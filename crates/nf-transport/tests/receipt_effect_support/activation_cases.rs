//! Genuine Finished ResponseSent and final current SQL read; no supplied event/session.
use super::Fixture;
use futures::future::poll_fn;
use nf_transport::{
    PeerError,
    receipt_effects::{ReceiptLane, ReceiptLaneEvent, lane::activation_test},
};
use std::task::Poll;
pub(super) async fn refused(f: &mut Fixture, expired: bool) {
    let mut server = ReceiptLane::server(&f.server).unwrap();
    let mut client = ReceiptLane::client(&f.client, 0).unwrap();
    server
        .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = loop {
        if let Some(ReceiptLaneEvent::Listening(address)) =
            poll_fn(|cx| server.poll(cx, &mut f.server)).await.unwrap()
        {
            break address;
        }
    };
    client.dial(address).unwrap();
    let mut first = false;
    loop {
        let (is_server, event) = poll_fn(|cx| {
            first = !first;
            if first {
                if let Poll::Ready(v) = server.poll(cx, &mut f.server) {
                    return Poll::Ready((true, v));
                }
                if let Poll::Ready(v) = client.poll(cx, &mut f.client) {
                    return Poll::Ready((false, v));
                }
            } else {
                if let Poll::Ready(v) = client.poll(cx, &mut f.client) {
                    return Poll::Ready((false, v));
                }
                if let Poll::Ready(v) = server.poll(cx, &mut f.server) {
                    return Poll::Ready((true, v));
                }
            }
            Poll::Pending
        })
        .await;
        if is_server {
            match event {
                Err(e) => {
                    assert!(
                        activation_test::reached(),
                        "genuine Finished completion boundary"
                    );
                    if expired {
                        assert!(matches!(e, PeerError::Replay));
                    } else {
                        assert!(matches!(e, PeerError::Policy | PeerError::Unauthorized));
                    }
                    break;
                }
                Ok(Some(ReceiptLaneEvent::Authenticated { .. })) => {
                    panic!("final SQL capture reported stale original activation")
                }
                Ok(Some(ReceiptLaneEvent::Closed { .. })) => panic!("unexpected early close"),
                Ok(_) => {}
            }
        } else {
            // A received Finished cannot be unsent; server reporting remains separately fenced.
            assert!(event.is_ok(), "valid client path");
        }
    }
    assert!(matches!(
        poll_fn(|cx| server.poll(cx, &mut f.server)).await,
        Err(PeerError::Offline)
    ));
    assert_eq!(server.pending_encoded(), (0, 0, 0));
    assert_eq!(f.client.book_anchors()[0].generation, 0);
    assert!(f.server.book_anchors().is_empty());
    client.shutdown();
}
