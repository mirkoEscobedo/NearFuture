use super::support::repo::RepoFixture;
use crate::{
    PeerError,
    notification::NotifyLimits,
    notification_effects::{
        NotifyLane, NotifyLaneEvent,
        lane::completion_test::{self, Cut},
    },
    receipt::SourceMinima,
};
use futures::future::poll_fn;
use std::{
    task::{Context, Poll},
    time::Duration,
};
struct Pair {
    server: NotifyLane,
    client: NotifyLane,
}
impl Pair {
    async fn new(f: &mut RepoFixture) -> Self {
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
        let mut pair = Self {
            server: NotifyLane::server(&f.repo, NotifyLimits::default()).unwrap(),
            client: NotifyLane::client(&f.client_repo, 0, 30, NotifyLimits::default()).unwrap(),
        };
        pair.server
            .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let address = tokio::time::timeout(
            Duration::from_secs(5),
            poll_fn(|cx| match pair.server.poll(cx, &mut f.repo) {
                Poll::Ready(Ok(Some(NotifyLaneEvent::Listening(a)))) => Poll::Ready(a),
                Poll::Ready(Err(e)) => panic!("actual listener: {e:?}"),
                _ => Poll::Pending,
            }),
        )
        .await
        .unwrap();
        pair.client.dial(address).unwrap();
        pair
    }
    async fn active(&mut self, f: &mut RepoFixture) {
        let mut subscribed = false;
        tokio::time::timeout(
            Duration::from_secs(5),
            poll_fn(|cx| {
                for (lane, repo) in [
                    (&mut self.server, &mut f.repo),
                    (&mut self.client, &mut f.client_repo),
                ] {
                    match lane.poll(cx, repo) {
                        Poll::Ready(Ok(Some(NotifyLaneEvent::Subscribed))) => subscribed = true,
                        Poll::Ready(Err(e)) => panic!("actual setup: {e:?}"),
                        _ => {}
                    }
                }
                if subscribed
                    && self.server.pending_encoded().0 == 0
                    && self.client.pending_encoded().0 == 0
                {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }),
        )
        .await
        .unwrap();
    }
    async fn probe(
        &mut self,
        f: &mut RepoFixture,
        cut: Cut,
    ) -> Result<Option<NotifyLaneEvent>, PeerError> {
        tokio::time::timeout(
            Duration::from_secs(15),
            poll_fn(|cx| {
                let server = self.server.poll(cx, &mut f.repo);
                if completion_test::reached() && cut != Cut::ClientSubscribed {
                    return match server {
                        Poll::Ready(r) => Poll::Ready(r),
                        _ => panic!("actual completion must return"),
                    };
                }
                if let Poll::Ready(Err(e)) = server {
                    panic!("failure before final boundary: {e:?}");
                }
                let client = self.client.poll(cx, &mut f.client_repo);
                if completion_test::reached() && cut == Cut::ClientSubscribed {
                    return match client {
                        Poll::Ready(r) => Poll::Ready(r),
                        _ => panic!("actual completion must return"),
                    };
                }
                if let Poll::Ready(Err(e)) = client {
                    panic!("failure before final boundary: {e:?}");
                }
                cx.waker().wake_by_ref();
                Poll::Pending
            }),
        )
        .await
        .unwrap()
    }
    fn shutdown(&mut self) {
        self.server.shutdown();
        self.client.shutdown();
    }
}
fn view(f: &mut RepoFixture) -> (SourceMinima, SourceMinima, [u8; 32]) {
    let server = f.repo.with_current_read(|cut| Ok(cut.source)).unwrap();
    let client = f
        .client_repo
        .with_current_read(|cut| Ok(cut.source))
        .unwrap();
    let head = f
        .client_repo
        .recover_observations()
        .unwrap()
        .head(0)
        .unwrap()
        .digest()
        .unwrap();
    (server, client, head)
}
async fn exercise(cut: Cut) {
    let mut f = RepoFixture::new();
    let mut pair = Pair::new(&mut f).await;
    if cut == Cut::ServerAck {
        pair.active(&mut f).await;
        f.repo
            .commit_trusted_prepared(f.commit.take().unwrap())
            .unwrap();
        pair.server.observe_retained(&mut f.repo).unwrap();
    }
    let before = view(&mut f);
    let _armed = completion_test::arm(cut);
    let result = pair.probe(&mut f, cut).await;
    assert!(
        completion_test::reached(),
        "delay must reach the actual final validated SQL boundary"
    );
    if cut == Cut::ServerOriginGap {
        assert_eq!(
            completion_test::origin_parity(),
            (true, true),
            "original expired while later owner timer remains live"
        );
    }
    assert!(
        matches!(result, Err(PeerError::Replay)),
        "original operation 5s must refuse completion despite valid active 30s: {result:?}"
    );
    let target = if cut == Cut::ClientSubscribed {
        &mut pair.client
    } else {
        &mut pair.server
    };
    let repo = if cut == Cut::ClientSubscribed {
        &mut f.client_repo
    } else {
        &mut f.repo
    };
    let mut cx = Context::from_waker(futures::task::noop_waker_ref());
    assert!(matches!(
        target.poll(&mut cx, repo),
        Poll::Ready(Err(PeerError::Offline))
    ));
    assert_eq!(target.pending_encoded(), (0, 0, 0));
    assert_eq!(
        view(&mut f),
        before,
        "completion refusal must not write Store or receipt book"
    );
    pair.shutdown();
}
#[tokio::test]
async fn client_subscribed_final_sql_preserves_original_pending_five_seconds() {
    exercise(Cut::ClientSubscribed).await;
}
#[tokio::test]
async fn server_subscribed_flush_final_sql_preserves_original_pending_five_seconds() {
    exercise(Cut::ServerSubscribed).await;
}
#[tokio::test]
async fn server_notice_ack_final_sql_preserves_original_pending_five_seconds() {
    exercise(Cut::ServerAck).await;
}

#[tokio::test]
async fn server_original_challenge_end_is_not_renewed_by_later_flush_timer() {
    exercise(Cut::ServerOriginGap).await;
}
