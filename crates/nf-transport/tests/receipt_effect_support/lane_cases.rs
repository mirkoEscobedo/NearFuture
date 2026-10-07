use super::fixture::Fixture;
use futures::future::poll_fn;
use nf_transport::{
    PeerError,
    receipt::{ReceiptPhase, SourceMinima},
    receipt_effects::*,
};
use std::{task::Poll, time::Duration};
async fn turn(
    server: &mut ReceiptLane,
    client: &mut ReceiptLane,
    fixture: &mut Fixture,
    first: &mut bool,
) -> (bool, Option<ReceiptLaneEvent>) {
    poll_fn(|cx| {
        *first = !*first;
        if *first {
            if let Poll::Ready(event) = server.poll(cx, &mut fixture.server) {
                return Poll::Ready((true, event.unwrap()));
            }
            if let Poll::Ready(event) = client.poll(cx, &mut fixture.client) {
                return Poll::Ready((false, event.unwrap()));
            }
        } else {
            if let Poll::Ready(event) = client.poll(cx, &mut fixture.client) {
                return Poll::Ready((false, event.unwrap()));
            }
            if let Poll::Ready(event) = server.poll(cx, &mut fixture.server) {
                return Poll::Ready((true, event.unwrap()));
            }
        }
        Poll::Pending
    })
    .await
}
async fn connected(fixture: &mut Fixture) -> (ReceiptLane, ReceiptLane) {
    let mut server = ReceiptLane::server(&fixture.server).unwrap();
    let mut client = ReceiptLane::client(&fixture.client, 0).unwrap();
    server
        .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = loop {
        let event = poll_fn(|cx| server.poll(cx, &mut fixture.server))
            .await
            .unwrap();
        if let Some(ReceiptLaneEvent::Listening(address)) = event {
            break address;
        }
    };
    client.dial(address).unwrap();
    let (mut server_active, mut client_active, mut first) = (false, false, false);
    while !server_active || !client_active {
        let (is_server, event) = turn(&mut server, &mut client, fixture, &mut first).await;
        if matches!(event, Some(ReceiptLaneEvent::Authenticated { .. })) {
            if is_server {
                server_active = true;
            } else {
                client_active = true;
            }
        }
    }
    assert_eq!(server.pending_encoded(), (0, 0, 0));
    assert_eq!(client.pending_encoded(), (0, 0, 0));
    (server, client)
}
async fn query(
    server: &mut ReceiptLane,
    client: &mut ReceiptLane,
    fixture: &mut Fixture,
) -> AdmittedReceipt {
    client.request_receipt(&mut fixture.client).unwrap();
    assert_eq!(client.pending_encoded().0, 1);
    let (mut accepted, mut first) = (None, false);
    while accepted.is_none() || server.pending_encoded().0 != 0 {
        let (is_server, event) = turn(server, client, fixture, &mut first).await;
        match event {
            Some(ReceiptLaneEvent::Accepted {
                outcome: receipt, ..
            }) => {
                assert!(!is_server);
                accepted = Some(receipt);
            }
            Some(ReceiptLaneEvent::Authenticated { .. }) => {
                panic!("same-session query must not renegotiate")
            }
            Some(ReceiptLaneEvent::Closed { .. }) => panic!("live receipt lane closed"),
            _ => {}
        }
    }
    assert_eq!(client.pending_encoded(), (0, 0, 0));
    accepted.unwrap()
}
#[tokio::test]
async fn actual_same_session_noise_receipts_activate_on_completion_and_retry_one_commit() {
    let mut fixture = Fixture::new(); // Real ACL/key/SQL setup is outside the protocol timer.
    tokio::time::timeout(Duration::from_secs(25), async {
        let (mut server, mut client) = connected(&mut fixture).await;
        let AdmittedReceipt::Status(status) = query(&mut server, &mut client, &mut fixture).await
        else {
            panic!("pending status")
        };
        assert_eq!(status.phase, ReceiptPhase::Pending);
        let ack = fixture
            .server
            .commit_trusted_prepared(fixture.commit.take().unwrap())
            .unwrap();
        assert_eq!(ack.sequence().0, 1);
        let bytes = std::fs::read(fixture.scratch.root.join("server.sqlite")).unwrap();
        for _ in 0..2 {
            let AdmittedReceipt::Status(status) =
                query(&mut server, &mut client, &mut fixture).await
            else {
                panic!("committed status")
            };
            assert_eq!(
                status.phase,
                ReceiptPhase::Committed {
                    sequence: ack.sequence()
                }
            );
            assert_eq!(status.operation, fixture.original.operation());
            assert_eq!(status.binding, fixture.original.binding_digest());
            assert_eq!(fixture.client.book_anchors()[0].generation, 2);
            assert_eq!(
                std::fs::read(fixture.scratch.root.join("server.sqlite")).unwrap(),
                bytes
            );
        }
        server.shutdown();
        client.shutdown();
    })
    .await
    .expect("owned paired receipt deadline");
}
#[tokio::test]
async fn actual_queued_receipt_policy_change_is_fenced_before_backend_poll() {
    let mut fixture = Fixture::new();
    tokio::time::timeout(Duration::from_secs(20), async {
        let (mut server, mut client) = connected(&mut fixture).await;
        client.request_receipt(&mut fixture.client).unwrap();
        let change = nf_identity::model::DeviceRevocation {
            scope: fixture.state.scope,
            issuer: fixture.server_public.account,
            device: fixture.client_public.device,
            frontier: fixture.state.revision,
        };
        let signature = fixture
            .owner_key
            .sign(&nf_identity::rotation::revocation_digest(&change));
        fixture.client.revoke_trusted(change, signature).unwrap();
        let result = poll_fn(|cx| client.poll(cx, &mut fixture.client)).await;
        assert!(matches!(
            result,
            Err(PeerError::Unauthorized | PeerError::Policy)
        ));
        assert_eq!(client.pending_encoded(), (0, 0, 0));
        assert_eq!(fixture.client.book_anchors()[0].generation, 0);
        server.shutdown();
    })
    .await
    .expect("owned paired policy deadline");
}
#[tokio::test]
async fn actual_queued_receipt_expiry_wakes_and_consumes_before_backend_poll() {
    let mut fixture = Fixture::new();
    tokio::time::timeout(Duration::from_secs(20), async {
        let (mut server, mut client) = connected(&mut fixture).await;
        client.request_receipt(&mut fixture.client).unwrap();
        tokio::time::sleep(Duration::from_millis(5050)).await;
        assert!(matches!(
            poll_fn(|cx| client.poll(cx, &mut fixture.client)).await,
            Err(PeerError::Replay)
        ));
        assert_eq!(client.pending_encoded(), (0, 0, 0));
        assert_eq!(fixture.client.book_anchors()[0].generation, 0);
        assert_eq!(
            fixture.client.book_anchors()[0].minimum,
            SourceMinima {
                event: nf_contract::identity::EventSeq(0),
                store_revision: 0,
                membership_revision: 1
            }
        );
        server.shutdown();
    })
    .await
    .expect("owned paired expiry deadline");
}

#[tokio::test]
async fn after_read_lane_expiry_refuses_before_actual_backend_poll() {
    let mut fixture = Fixture::new();
    tokio::time::timeout(Duration::from_secs(20), async {
        let (mut server, mut client) = connected(&mut fixture).await;
        client.request_receipt(&mut fixture.client).unwrap();
        // First read checks active policy; the next read guards the queued actual request.
        let _delay = ReceiptRepo::delay_current_read_for_test(1);
        let result = poll_fn(|cx| client.poll(cx, &mut fixture.client)).await;
        assert!(matches!(result, Err(PeerError::Replay)));
        assert_eq!(client.pending_encoded(), (0, 0, 0));
        assert_eq!(fixture.client.book_anchors()[0].generation, 0);
        server.shutdown();
    })
    .await
    .expect("owned after-read refusal deadline");
}
