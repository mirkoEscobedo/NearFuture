use super::repo::RepoFixture;
use futures::future::poll_fn;
use nf_transport::{
    notification::NotifyLimits,
    notification_effects::{NotifyLane, NotifyLaneEvent},
    receipt::SourceMinima,
    receipt_effects::{AdmittedReceipt, ReceiptCompletion, ReceiptLane, ReceiptLaneEvent},
};
use std::{
    task::{Context, Poll},
    time::Duration,
};
pub struct Lanes {
    pub ns: NotifyLane,
    pub nc: NotifyLane,
    pub rs: ReceiptLane,
    pub rc: ReceiptLane,
}
fn observe<T>(p: Poll<Result<Option<T>, nf_transport::PeerError>>) -> Option<T> {
    match p {
        Poll::Ready(Ok(value)) => value,
        Poll::Ready(Err(e)) => panic!("actual owned backend: {e:?}"),
        Poll::Pending => None,
    }
}
impl Lanes {
    pub async fn new(f: &mut RepoFixture) -> Self {
        Self::with_lifetime(f, 30).await
    }
    pub async fn with_lifetime(f: &mut RepoFixture, lifetime: u16) -> Self {
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
        let mut lanes = Self {
            ns: NotifyLane::server(&f.repo, NotifyLimits::default()).unwrap(),
            nc: NotifyLane::client(&f.client_repo, 0, lifetime, NotifyLimits::default()).unwrap(),
            rs: ReceiptLane::server(&f.repo).unwrap(),
            rc: ReceiptLane::client(&f.client_repo, 0).unwrap(),
        };
        lanes
            .ns
            .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        lanes
            .rs
            .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let mut notify_address = None;
        let mut receipt_address = None;
        tokio::time::timeout(
            Duration::from_secs(5),
            poll_fn(|cx| {
                if let Some(NotifyLaneEvent::Listening(a)) = observe(lanes.ns.poll(cx, &mut f.repo))
                {
                    notify_address = Some(a);
                }
                if let Some(ReceiptLaneEvent::Listening(a)) =
                    observe(lanes.rs.poll(cx, &mut f.repo))
                {
                    receipt_address = Some(a);
                }
                if notify_address.is_some() && receipt_address.is_some() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }),
        )
        .await
        .unwrap();
        lanes.nc.dial(notify_address.unwrap()).unwrap();
        lanes.rc.dial(receipt_address.unwrap()).unwrap();
        let mut subscribed = false;
        let mut receipt_authenticated = false;
        tokio::time::timeout(
            Duration::from_secs(5),
            poll_fn(|cx| {
                observe(lanes.ns.poll(cx, &mut f.repo));
                if let Some(NotifyLaneEvent::Subscribed) =
                    observe(lanes.nc.poll(cx, &mut f.client_repo))
                {
                    subscribed = true;
                }
                observe(lanes.rs.poll(cx, &mut f.repo));
                if let Some(ReceiptLaneEvent::Authenticated { .. }) =
                    observe(lanes.rc.poll(cx, &mut f.client_repo))
                {
                    receipt_authenticated = true;
                }
                if subscribed && receipt_authenticated {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }),
        )
        .await
        .unwrap();
        lanes
    }
    fn poll_all(
        &mut self,
        cx: &mut Context<'_>,
        f: &mut RepoFixture,
    ) -> (Option<NotifyLaneEvent>, Option<ReceiptLaneEvent>) {
        observe(self.ns.poll(cx, &mut f.repo));
        let notify = observe(self.nc.poll(cx, &mut f.client_repo));
        observe(self.rs.poll(cx, &mut f.repo));
        let receipt = observe(self.rc.poll(cx, &mut f.client_repo));
        (notify, receipt)
    }
    pub async fn receipt(&mut self, f: &mut RepoFixture) -> (AdmittedReceipt, ReceiptCompletion) {
        tokio::time::timeout(
            Duration::from_secs(5),
            poll_fn(|cx| {
                if let (
                    _,
                    Some(ReceiptLaneEvent::Accepted {
                        outcome,
                        completion: Some(completion),
                    }),
                ) = self.poll_all(cx, f)
                {
                    Poll::Ready((outcome, *completion))
                } else {
                    Poll::Pending
                }
            }),
        )
        .await
        .unwrap()
    }
    pub async fn dirty_notice(&mut self, f: &mut RepoFixture) {
        tokio::time::timeout(
            Duration::from_secs(5),
            poll_fn(|cx| {
                if let (Some(NotifyLaneEvent::Dirty), _) = self.poll_all(cx, f) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }),
        )
        .await
        .unwrap();
    }
    pub fn shutdown(&mut self) {
        self.ns.shutdown();
        self.nc.shutdown();
        self.rs.shutdown();
        self.rc.shutdown();
    }
}
