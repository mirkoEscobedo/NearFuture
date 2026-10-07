mod notification_effect_support;
use futures::future::poll_fn;
use nf_transport::{
    notification::NotifyLimits,
    notification_effects::{NotifyLane, NotifyLaneEvent},
    receipt::SourceMinima,
};
use notification_effect_support::repo::RepoFixture;
async fn exercise(limits: NotifyLimits) {
    let mut f = RepoFixture::new();
    f.client_repo
        .initialize_originals(&[(
            0,
            f.original.clone(),
            SourceMinima {
                store_revision: 0,
                event: nf_contract::identity::EventSeq(0),
                membership_revision: 1,
            },
        )])
        .unwrap();
    let mut server = NotifyLane::server(&f.repo, limits).unwrap();
    let mut client = NotifyLane::client(&f.client_repo, 0, 30, limits).unwrap();
    server
        .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        poll_fn(|cx| match server.poll(cx, &mut f.repo) {
            std::task::Poll::Ready(Ok(Some(NotifyLaneEvent::Listening(a)))) => {
                std::task::Poll::Ready(a)
            }
            std::task::Poll::Ready(Err(e)) => panic!("owned server poll: {e:?}"),
            _ => std::task::Poll::Pending,
        }),
    )
    .await
    .unwrap();
    client.dial(address).unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        poll_fn(|cx| {
            for (lane, repo) in [
                (&mut server, &mut f.repo),
                (&mut client, &mut f.client_repo),
            ] {
                match lane.poll(cx, repo) {
                    std::task::Poll::Ready(Ok(Some(NotifyLaneEvent::Subscribed))) => {
                        return std::task::Poll::Ready(());
                    }
                    std::task::Poll::Ready(Err(e)) => panic!("owned lane: {e:?}"),
                    _ => {}
                }
            }
            std::task::Poll::Pending
        }),
    )
    .await
    .unwrap();
    f.repo
        .commit_trusted_prepared(f.commit.take().unwrap())
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        poll_fn(|cx| {
            for (lane, repo) in [
                (&mut server, &mut f.repo),
                (&mut client, &mut f.client_repo),
            ] {
                match lane.poll(cx, repo) {
                    std::task::Poll::Ready(Ok(Some(NotifyLaneEvent::Dirty))) => {
                        return std::task::Poll::Ready(());
                    }
                    std::task::Poll::Ready(Err(e)) => panic!("owned lane: {e:?}"),
                    _ => {}
                }
            }
            std::task::Poll::Pending
        }),
    )
    .await
    .unwrap();
    server.shutdown();
    client.shutdown();
}

#[tokio::test]
async fn actual_paired_noise_backend_establishes_subscription_and_retained_dirty_hint() {
    exercise(NotifyLimits::default()).await;
}
#[tokio::test]
async fn one_item_negotiated_queue_progresses_by_releasing_exact_returned_request_first() {
    exercise(NotifyLimits {
        queue_items: 1,
        ..NotifyLimits::default()
    })
    .await;
}
