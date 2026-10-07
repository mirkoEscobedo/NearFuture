use super::support::repo::RepoFixture;
use libp2p::swarm::ConnectionId;
use nf_transport::{
    PeerError,
    notification::NotifyRecord,
    notification_effects::{
        NotifyBroker, NotifyClientHandshake, NotifyServerHandshake, NotifySubscriber,
    },
    receipt_effects::ReceiptRepo,
};
fn delayed<T>(skip: usize, call: impl FnOnce() -> Result<T, PeerError>) -> Result<T, PeerError> {
    let _delay = ReceiptRepo::delay_current_read_for_test(skip);
    let started = std::time::Instant::now();
    let result = call();
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(5100),
        "actual validated SQL scheduling delay must be reached"
    );
    result
}
fn connection() -> ConnectionId {
    ConnectionId::new_unchecked(7)
}
struct Subscription {
    f: RepoFixture,
    broker: NotifyBroker,
    client: NotifySubscriber,
    challenge: NotifyRecord,
}
impl Subscription {
    fn new(lifetime: u16) -> Self {
        let mut f = RepoFixture::new();
        let (server, client) = f.sessions();
        let mut broker = NotifyBroker::new(server).unwrap();
        let mut client = NotifySubscriber::new(client).unwrap();
        let begin = client
            .begin_subscription(f.original.clone(), lifetime, &mut f.client_repo)
            .unwrap();
        let challenge = broker
            .begin_subscription(begin, f.client_peer, connection(), &mut f.repo)
            .unwrap();
        Self {
            f,
            broker,
            client,
            challenge,
        }
    }
    fn proof(&mut self) -> NotifyRecord {
        self.client
            .challenge(
                self.challenge.clone(),
                self.f.server_peer,
                connection(),
                &mut self.f.client_repo,
            )
            .unwrap()
    }
    fn subscribed(&mut self) -> NotifyRecord {
        let proof = self.proof();
        self.broker
            .prove_subscription(proof, self.f.client_peer, connection(), &mut self.f.repo)
            .unwrap()
    }
    fn activate(&mut self) {
        let subscribed = self.subscribed();
        self.client
            .subscribed(
                subscribed.clone(),
                self.f.server_peer,
                connection(),
                &mut self.f.client_repo,
            )
            .unwrap();
        let permit = self
            .broker
            .prepare_output(
                subscribed,
                self.f.client_peer,
                connection(),
                &mut self.f.repo,
            )
            .unwrap();
        self.broker
            .complete_emission(permit, self.f.client_peer, connection(), &mut self.f.repo)
            .unwrap();
    }
    fn notice(&mut self) -> NotifyRecord {
        self.f
            .repo
            .commit_trusted_prepared(self.f.commit.take().unwrap())
            .unwrap();
        self.broker
            .next_notice(self.f.client_peer, connection(), &mut self.f.repo)
            .unwrap()
            .unwrap()
    }
}
#[test]
fn client_challenge_refuses_signing_after_current_sql_consumes_own_begin() {
    let mut s = Subscription::new(4);
    assert!(matches!(
        delayed(1, || s.client.challenge(
            s.challenge,
            s.f.server_peer,
            connection(),
            &mut s.f.client_repo
        )),
        Err(PeerError::Replay)
    ));
}
#[test]
fn client_subscribed_refuses_activation_after_current_sql_consumes_own_begin() {
    let mut s = Subscription::new(4);
    let record = s.subscribed();
    assert!(matches!(
        delayed(0, || s.client.subscribed(
            record,
            s.f.server_peer,
            connection(),
            &mut s.f.client_repo
        )),
        Err(PeerError::Replay)
    ));
}
#[test]
fn server_prove_refuses_admission_after_current_sql_consumes_challenge() {
    let mut s = Subscription::new(30);
    let proof = s.proof();
    assert!(matches!(
        delayed(2, || s.broker.prove_subscription(
            proof,
            s.f.client_peer,
            connection(),
            &mut s.f.repo
        )),
        Err(PeerError::Replay)
    ));
}
#[test]
fn client_notice_refuses_ack_signing_after_current_sql_consumes_lifetime() {
    let mut s = Subscription::new(4);
    s.activate();
    let notice = s.notice();
    assert!(matches!(
        delayed(1, || s.client.notice(
            notice,
            s.f.server_peer,
            connection(),
            &mut s.f.client_repo
        )),
        Err(PeerError::Replay)
    ));
    assert!(s.client.dirty());
}
#[test]
fn server_ack_refuses_completion_after_current_sql_consumes_awaited_deadline() {
    let mut s = Subscription::new(30);
    s.activate();
    let notice = s.notice();
    let ack = s
        .client
        .notice(notice, s.f.server_peer, connection(), &mut s.f.client_repo)
        .unwrap();
    assert!(matches!(
        delayed(0, || s.broker.accept_ack(
            ack,
            s.f.client_peer,
            connection(),
            &mut s.f.repo
        )),
        Err(PeerError::Replay)
    ));
}
#[test]
fn subscribed_flush_refuses_transition_after_current_sql_consumes_lifetime() {
    let mut s = Subscription::new(4);
    let record = s.subscribed();
    let emission = s
        .broker
        .prepare_output(record, s.f.client_peer, connection(), &mut s.f.repo)
        .unwrap();
    assert!(matches!(
        delayed(0, || s.broker.complete_emission(
            emission,
            s.f.client_peer,
            connection(),
            &mut s.f.repo
        )),
        Err(PeerError::Replay)
    ));
}
#[test]
fn initial_hello_output_refuses_after_current_sql_consumes_five_seconds() {
    let mut f = RepoFixture::new();
    let (mut client, hello) = NotifyClientHandshake::begin(
        f.client.public.clone(),
        f.client_peer,
        f.original.source(),
        f.policy(),
    )
    .unwrap();
    assert!(matches!(
        delayed(0, || client.prepare_output(
            hello,
            f.server_peer,
            connection(),
            &mut f.client_repo
        )),
        Err(PeerError::Replay)
    ));
}
#[test]
fn pending_server_output_refuses_after_current_sql_consumes_five_seconds() {
    let mut f = RepoFixture::new();
    let (_, hello) = NotifyClientHandshake::begin(
        f.client.public.clone(),
        f.client_peer,
        f.original.source(),
        f.policy(),
    )
    .unwrap();
    let (mut server, reply) = NotifyServerHandshake::begin(
        hello,
        f.client_peer,
        connection(),
        f.server.public.clone(),
        f.server_peer,
        f.policy(),
        &f.state,
        &f.server.device_key,
    )
    .unwrap();
    assert!(matches!(
        delayed(0, || server.prepare_output(
            reply,
            f.client_peer,
            connection(),
            &mut f.repo
        )),
        Err(PeerError::Replay)
    ));
}

#[test]
fn server_notice_refuses_signing_after_final_current_sql_consumes_lifetime() {
    let mut s = Subscription::new(4);
    s.activate();
    s.f.repo
        .commit_trusted_prepared(s.f.commit.take().unwrap())
        .unwrap();
    assert!(matches!(
        delayed(2, || s.broker.next_notice(
            s.f.client_peer,
            connection(),
            &mut s.f.repo
        )),
        Err(PeerError::Replay)
    ));
}
#[tokio::test]
async fn actual_lane_refuses_notice_enqueue_after_last_current_sql_consumes_lifetime() {
    use super::support::lanes::Lanes;
    use futures::future::poll_fn;
    use std::{task::Poll, time::Duration};
    let mut f = RepoFixture::new();
    let mut l = Lanes::with_lifetime(&mut f, 4).await;
    tokio::time::timeout(
        Duration::from_secs(5),
        poll_fn(|cx| {
            if let Poll::Ready(Err(e)) = l.ns.poll(cx, &mut f.repo) {
                panic!("real server flush: {e:?}");
            }
            if let Poll::Ready(Err(e)) = l.nc.poll(cx, &mut f.client_repo) {
                panic!("real subscriber: {e:?}");
            }
            if l.ns.pending_encoded().0 == 0 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await
    .unwrap();
    f.repo
        .commit_trusted_prepared(f.commit.take().unwrap())
        .unwrap();
    let result = delayed(7, || l.ns.observe_retained(&mut f.repo));
    assert!(
        matches!(result, Err(PeerError::Replay)),
        "must refuse actual backend enqueue after expired final SQL cut"
    );
    assert_eq!(l.ns.pending_encoded().0, 0);
    l.shutdown();
}
#[test]
fn active_thirty_second_subscription_does_not_reuse_handshake_five_second_cut() {
    let mut s = Subscription::new(30);
    s.activate();
    s.f.repo
        .commit_trusted_prepared(s.f.commit.take().unwrap())
        .unwrap();
    assert!(matches!(
        delayed(2, || s.broker.next_notice(
            s.f.client_peer,
            connection(),
            &mut s.f.repo
        )),
        Ok(Some(_))
    ));
}
#[tokio::test]
async fn actual_followup_refuses_if_receipt_guard_work_consumes_notification_lifetime() {
    use super::support::lanes::Lanes;
    let mut f = RepoFixture::new();
    let mut l = Lanes::with_lifetime(&mut f, 4).await;
    let result = delayed(2, || l.nc.request_followup(&mut l.rc, &mut f.client_repo));
    assert!(
        matches!(result, Err(PeerError::Replay)),
        "expired notification must not report successful followup issuance"
    );
    assert!(l.nc.dirty());
    l.shutdown();
}
