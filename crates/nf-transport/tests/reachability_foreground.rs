#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/scratch.rs"]
mod scratch;
use nf_contract::identity::RequestId;
use nf_transport::{
    PeerError,
    auth::ServerPin,
    node::{PeerServer, PeerServerEvent, request_status_diagnosed},
    query::QueryResult,
    reachability::{EndpointRefusal, Failure, Observation},
    records::{PeerLimits, RetainedPhase},
    session::SessionPolicy,
};
use owned_community::OwnedCommunity;
use std::{net::TcpListener, time::Duration};

fn policy_and_pin(c: &OwnedCommunity) -> (SessionPolicy, ServerPin) {
    (
        SessionPolicy {
            scope: c.state.scope,
            ruleset: [4; 32],
            content: [5; 32],
            limits: PeerLimits::default(),
            minimum_membership: 1,
        },
        ServerPin {
            peer: libp2p::PeerId::from_bytes(&c.server.public.peer).unwrap(),
            account: c.server.public.account,
            device: c.server.public.device,
            minimum_membership: 1,
        },
    )
}

#[tokio::test(flavor = "current_thread")]
async fn diagnostics_record_an_actual_authenticated_reply() {
    let tmp = scratch::Scratch::new();
    let c = OwnedCommunity::new(&tmp.0);
    let (policy, pin) = policy_and_pin(&c);
    let mut server = PeerServer::new(c.server_store, &c.server_vault, policy).unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        let address = loop {
            if let PeerServerEvent::Ready { control, .. } = server.next().await.unwrap() {
                break control;
            }
        };
        let client = request_status_diagnosed(
            c.client_store,
            &c.client_vault,
            policy,
            pin,
            Some(address),
            RequestId::from_bytes([9; 16]),
        );
        tokio::pin!(client);
        loop {
            tokio::select! {
                report = &mut client => {
                    assert_eq!(report.result, Ok(QueryResult::Status(RetainedPhase::UnknownRequest)));
                    assert_eq!(report.diagnostic.observation(), Observation::ReplyValidated);
                    assert_eq!(report.diagnostic.failure(), None);
                    break;
                },
                event = server.next() => { event.unwrap(); }
            }
        }
    }).await.expect("actual authenticated query deadline");
}

#[tokio::test(flavor = "current_thread")]
async fn a_listening_endpoint_without_noise_hits_the_existing_deadline() {
    let tmp = scratch::Scratch::new();
    let c = OwnedCommunity::new(&tmp.0);
    let (policy, pin) = policy_and_pin(&c);
    // Keep the listener alive without accepting or speaking Noise. No closed-port race.
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = format!(
        "/ip4/127.0.0.1/tcp/{}/p2p/{}",
        listener.local_addr().unwrap().port(),
        pin.peer
    )
    .parse()
    .unwrap();
    let report = tokio::time::timeout(
        Duration::from_secs(10),
        request_status_diagnosed(
            c.client_store,
            &c.client_vault,
            policy,
            pin,
            Some(address),
            RequestId::from_bytes([9; 16]),
        ),
    )
    .await
    .expect("diagnosed query outer deadline");
    assert_eq!(report.result, Err(PeerError::Offline));
    assert_eq!(report.diagnostic.observation(), Observation::DialAttempted);
    assert_eq!(report.diagnostic.failure(), Some(Failure::Deadline));
    drop(listener);
}

#[tokio::test(flavor = "current_thread")]
async fn diagnosed_remote_refusal_precedes_any_dial() {
    let tmp = scratch::Scratch::new();
    let c = OwnedCommunity::new(&tmp.0);
    let (policy, pin) = policy_and_pin(&c);
    let address = format!("/ip4/192.0.2.1/tcp/1234/p2p/{}", pin.peer)
        .parse()
        .unwrap();
    let report = request_status_diagnosed(
        c.client_store,
        &c.client_vault,
        policy,
        pin,
        Some(address),
        RequestId::from_bytes([9; 16]),
    )
    .await;
    assert_eq!(report.result, Err(PeerError::Unauthorized));
    assert_eq!(report.diagnostic.observation(), Observation::NotDialed);
    assert_eq!(
        report.diagnostic.failure(),
        Some(Failure::Endpoint(EndpointRefusal::RemoteRouteUnsupported))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn absent_explicit_configuration_stays_offline_without_dial() {
    let tmp = scratch::Scratch::new();
    let c = OwnedCommunity::new(&tmp.0);
    let (policy, pin) = policy_and_pin(&c);
    let report = request_status_diagnosed(
        c.client_store,
        &c.client_vault,
        policy,
        pin,
        None,
        RequestId::from_bytes([9; 16]),
    )
    .await;
    assert_eq!(report.result, Err(PeerError::Offline));
    assert_eq!(report.diagnostic.observation(), Observation::NotDialed);
    assert_eq!(
        report.diagnostic.failure(),
        Some(Failure::Endpoint(EndpointRefusal::NoConfiguredPeer))
    );
}
