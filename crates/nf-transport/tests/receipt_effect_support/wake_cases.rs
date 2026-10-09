//! Authentic receipt progress for an owner that suppresses internal lane events.
use super::{AdmittedReceipt, Fixture, ReceiptLaneEvent, ReceiptPhase, connected};
use futures::future::poll_fn;
use std::{task::Poll, time::Duration};

#[tokio::test]
async fn actual_retained_receipt_internal_progress_drives_pending_owner() {
    let mut fixture = Fixture::new();
    tokio::time::timeout(Duration::from_secs(25), async {
        let (mut server, mut client) = connected(&mut fixture).await;
        client.request_receipt(&mut fixture.client).unwrap();
        let mut accepted = None;
        let outcome = loop {
            let (server_result, client_result) = poll_fn(|cx| {
                // Both real lanes receive the owner waker. Drain actual Ready events;
                // wait for a genuine wake only when neither lane has ready progress.
                let server_result = server.poll(cx, &mut fixture.server);
                let client_result = client.poll(cx, &mut fixture.client);
                if server_result.is_pending() && client_result.is_pending() {
                    Poll::Pending
                } else {
                    Poll::Ready((server_result, client_result))
                }
            })
            .await;
            for result in [server_result, client_result] {
                match result {
                    Poll::Ready(Ok(Some(ReceiptLaneEvent::Accepted { outcome, .. }))) => {
                        assert!(accepted.replace(outcome).is_none());
                    }
                    Poll::Ready(Ok(Some(ReceiptLaneEvent::Closed { .. }))) => {
                        panic!("actual retained receipt lane closed");
                    }
                    Poll::Ready(Err(error)) => panic!("actual retained receipt lane: {error:?}"),
                    _ => {}
                }
            }
            if accepted.is_some() && server.pending_encoded().0 == 0 {
                break accepted.take().unwrap();
            }
        };
        let AdmittedReceipt::Status(status) = outcome else {
            panic!("actual retained pending receipt status");
        };
        assert_eq!(status.phase, ReceiptPhase::Pending);
        assert_eq!(status.operation, fixture.original.operation());
        assert_eq!(status.binding, fixture.original.binding_digest());
        assert_eq!(client.pending_encoded(), (0, 0, 0));
        assert_eq!(server.pending_encoded(), (0, 0, 0));
        server.shutdown();
        client.shutdown();
    })
    .await
    .expect("owned paired receipt deadline");
}
