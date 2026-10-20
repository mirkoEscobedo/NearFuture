use super::support as notification_effect_support;
use libp2p::swarm::ConnectionId;
use nf_transport::{
    PeerError,
    notification::{NotifyBody, NotifyRecord},
    notification_effects::{NotifyBroker, NotifySubscriber},
};
use notification_effect_support::repo::RepoFixture;
fn ready(f: &mut RepoFixture) -> (NotifyBroker, NotifySubscriber) {
    let c = ConnectionId::new_unchecked(7);
    let (server, client) = f.sessions();
    let mut broker = NotifyBroker::new(server).unwrap();
    let mut subscriber = NotifySubscriber::new(client).unwrap();
    let begin = subscriber
        .begin_subscription(f.original.clone(), 30, &mut f.client_repo)
        .unwrap();
    let challenge = broker
        .begin_subscription(begin, f.client_peer, c, &mut f.repo)
        .unwrap();
    let proof = subscriber
        .challenge(challenge, f.server_peer, c, &mut f.client_repo)
        .unwrap();
    let subscribed = broker
        .prove_subscription(proof, f.client_peer, c, &mut f.repo)
        .unwrap();
    subscriber
        .subscribed(subscribed.clone(), f.server_peer, c, &mut f.client_repo)
        .unwrap();
    let e = broker
        .prepare_output(subscribed, f.client_peer, c, &mut f.repo)
        .unwrap();
    broker
        .complete_emission(e, f.client_peer, c, &mut f.repo)
        .unwrap();
    (broker, subscriber)
}
fn committed_notice(f: &mut RepoFixture, b: &mut NotifyBroker) -> NotifyRecord {
    f.repo
        .commit_trusted_prepared(f.commit.take().unwrap())
        .unwrap();
    b.next_notice(f.client_peer, ConnectionId::new_unchecked(7), &mut f.repo)
        .unwrap()
        .unwrap()
}
#[test]
fn actual_signed_notice_sets_dirty_and_acknowledges_only_that_hint() {
    let mut f = RepoFixture::new();
    let (mut b, mut s) = ready(&mut f);
    let notice = committed_notice(&mut f, &mut b);
    let ack = s
        .notice(
            notice,
            f.server_peer,
            ConnectionId::new_unchecked(7),
            &mut f.client_repo,
        )
        .unwrap();
    assert!(s.dirty());
    assert!(matches!(
        ack.body,
        NotifyBody::NoticeAck { sequence: 1, .. }
    ));
    b.accept_ack(
        ack,
        f.client_peer,
        ConnectionId::new_unchecked(7),
        &mut f.repo,
    )
    .unwrap();
    assert!(
        b.next_notice(f.client_peer, ConnectionId::new_unchecked(7), &mut f.repo)
            .unwrap()
            .is_none()
    );
}
#[test]
fn duplicate_notice_never_signs_a_second_ack_and_keeps_dirty() {
    let mut f = RepoFixture::new();
    let (mut b, mut s) = ready(&mut f);
    let notice = committed_notice(&mut f, &mut b);
    s.notice(
        notice.clone(),
        f.server_peer,
        ConnectionId::new_unchecked(7),
        &mut f.client_repo,
    )
    .unwrap();
    assert!(matches!(
        s.notice(
            notice,
            f.server_peer,
            ConnectionId::new_unchecked(7),
            &mut f.client_repo
        ),
        Err(PeerError::Replay)
    ));
    assert!(s.dirty());
}
#[test]
fn failed_ack_first_attempt_poisoning_prevents_retry() {
    let mut f = RepoFixture::new();
    let (mut b, mut s) = ready(&mut f);
    let notice = committed_notice(&mut f, &mut b);
    let ack = s
        .notice(
            notice,
            f.server_peer,
            ConnectionId::new_unchecked(7),
            &mut f.client_repo,
        )
        .unwrap();
    assert!(matches!(
        b.accept_ack(
            ack.clone(),
            f.client_peer,
            ConnectionId::new_unchecked(8),
            &mut f.repo
        ),
        Err(PeerError::Session)
    ));
    assert!(matches!(
        b.accept_ack(
            ack,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo
        ),
        Err(PeerError::Replay)
    ));
}
