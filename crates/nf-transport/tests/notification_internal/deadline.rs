use super::support::repo::RepoFixture;
use futures::future::poll_fn;
use nf_transport::{
    notification::NotifyLimits,
    notification_effects::{NotifyLane, NotifyLaneEvent},
    receipt::SourceMinima,
};
use std::{task::Poll, time::Duration};
#[tokio::test]
async fn accepted_subscription_refuses_after_real_sql_cut_resumes_past_private_lifetime() {
    let mut f = RepoFixture::new();
    f.client_repo
        .initialize_originals(&[(
            0,
            f.original.clone(),
            SourceMinima {
                event: nf_contract::identity::EventSeq(0),
                store_revision: 0,
                membership_revision: 1,
            },
        )])
        .unwrap();
    let mut server = NotifyLane::server(&f.repo, NotifyLimits::default()).unwrap();
    let mut client = NotifyLane::client(&f.client_repo, 0, 30, NotifyLimits::default()).unwrap();
    server
        .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = tokio::time::timeout(
        Duration::from_secs(5),
        poll_fn(|cx| match server.poll(cx, &mut f.repo) {
            Poll::Ready(Ok(Some(NotifyLaneEvent::Listening(a)))) => Poll::Ready(a),
            Poll::Ready(Err(e)) => panic!("real listener: {e:?}"),
            _ => Poll::Pending,
        }),
    )
    .await
    .unwrap();
    client.dial(address).unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        poll_fn(|cx| {
            if let Poll::Ready(Err(e)) = server.poll(cx, &mut f.repo) {
                panic!("real server: {e:?}");
            }
            match client.poll(cx, &mut f.client_repo) {
                Poll::Ready(Ok(Some(NotifyLaneEvent::Subscribed))) => Poll::Ready(()),
                Poll::Ready(Err(e)) => panic!("real subscriber: {e:?}"),
                _ => Poll::Pending,
            }
        }),
    )
    .await
    .unwrap();
    let remaining = client
        .remaining_for_test()
        .expect("actual own-Begin lifetime remains after subscription");
    assert!(!remaining.is_zero() && remaining <= Duration::from_secs(30));
    // Setup consumes the actual client-owned lifetime; this reads its remaining
    // duration without renewing its origin or changing production deadlines.
    // The hook receives no clock, key, record or Store parameter. It only consumes
    // real elapsed time after the ordinary fresh SQL/current-policy read returned.
    let mut observed = false;
    let result = poll_fn(|cx| {
        Poll::Ready(client.poll_after_read(cx, &mut f.client_repo, || {
            observed = true;
            std::thread::sleep(remaining + Duration::from_millis(50))
        }))
    })
    .await;
    assert!(
        observed,
        "private observer must actually consume real elapsed time"
    );
    assert!(
        matches!(result, Poll::Ready(Err(nf_transport::PeerError::Replay))),
        "must refuse before another backend opportunity after elapsed read"
    );
    assert!(matches!(
        client.poll(
            &mut std::task::Context::from_waker(std::task::Waker::noop()),
            &mut f.client_repo
        ),
        Poll::Ready(Err(nf_transport::PeerError::Offline))
    ));
    server.shutdown();
}
