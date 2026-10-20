use super::support as notification_effect_support;
use libp2p::swarm::ConnectionId;
use nf_transport::{
    notification::NotifyBody,
    notification_effects::{NotifyBroker, NotifySubscriber},
};
use notification_effect_support::repo::RepoFixture;
#[test]
fn real_retained_transition_emits_once_only_after_subscribed_flush() {
    let mut f = RepoFixture::new();
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
    f.repo
        .commit_trusted_prepared(f.commit.take().unwrap())
        .unwrap();
    assert!(
        broker
            .next_notice(f.client_peer, c, &mut f.repo)
            .unwrap()
            .is_none()
    );
    let emission = broker
        .prepare_output(subscribed, f.client_peer, c, &mut f.repo)
        .unwrap();
    broker
        .complete_emission(emission, f.client_peer, c, &mut f.repo)
        .unwrap();
    let notice = broker
        .next_notice(f.client_peer, c, &mut f.repo)
        .unwrap()
        .expect("real retained change");
    assert!(matches!(
        notice.body,
        NotifyBody::Notice { sequence: 1, .. }
    ));
    assert!(
        broker
            .next_notice(f.client_peer, c, &mut f.repo)
            .unwrap()
            .is_none()
    );
}
