use super::support as notification_effect_support;
use libp2p::swarm::ConnectionId;
use nf_transport::{
    notification::{NotifyBody, NotifyRecord, NotifySelector},
    notification_effects::NotifyBroker,
};
use notification_effect_support::repo::RepoFixture;
#[test]
fn actual_retained_request_and_current_sql_issue_exact_subscription_challenge() {
    let mut f = RepoFixture::new();
    let (server, _) = f.sessions();
    let context = server.context();
    let mut broker = NotifyBroker::new(server).unwrap();
    let target = NotifySelector {
        request: f.original.request(),
        operation: f.original.operation(),
        binding: f.original.binding_digest(),
    };
    let begin = NotifyRecord {
        context,
        body: NotifyBody::BeginSubscribe {
            subscription: [8; 16],
            selector: target,
            nonce: [7; 32],
            minimum_membership: 1,
            lifetime: 30,
        },
    };
    let challenge = broker
        .begin_subscription(
            begin,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo,
        )
        .unwrap();
    assert!(matches!(
        challenge.body,
        NotifyBody::SubscribeChallenge {
            subscription: [8, ..],
            client_nonce: [7, ..],
            frontier: 1,
            ..
        }
    ));
}
fn begin(f: &mut RepoFixture) -> (NotifyBroker, NotifyRecord) {
    begin_with_lifetime(f, 30)
}
fn begin_with_lifetime(f: &mut RepoFixture, lifetime: u16) -> (NotifyBroker, NotifyRecord) {
    let (server, _) = f.sessions();
    let context = server.context();
    let mut broker = NotifyBroker::new(server).unwrap();
    let record = NotifyRecord {
        context,
        body: NotifyBody::BeginSubscribe {
            subscription: [8; 16],
            selector: NotifySelector {
                request: f.original.request(),
                operation: f.original.operation(),
                binding: f.original.binding_digest(),
            },
            nonce: [7; 32],
            minimum_membership: 1,
            lifetime,
        },
    };
    let reply = broker
        .begin_subscription(
            record,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo,
        )
        .unwrap();
    (broker, reply)
}
fn signed_proof(f: &RepoFixture, record: &NotifyRecord) -> NotifyRecord {
    use nf_identity::{model::DeviceProof, signing::device_digest};
    let NotifyBody::SubscribeChallenge {
        subscription,
        server_nonce,
        frontier,
        challenge,
        ..
    } = record.body
    else {
        panic!("actual challenge")
    };
    let mut proof = DeviceProof {
        scope: f.state.scope,
        account: f.client.public.account,
        device: f.client.public.device,
        frontier,
        peer: f.client_peer.to_bytes(),
        challenge,
        signature: [0; 64],
    };
    proof.signature = f.client.device_key.sign(&device_digest(&proof).unwrap());
    NotifyRecord {
        context: record.context,
        body: NotifyBody::ProveSubscribe {
            subscription,
            nonce: server_nonce,
            proof,
        },
    }
}
#[test]
fn real_signed_subscription_proof_returns_bound_subscribed() {
    let mut f = RepoFixture::new();
    let (mut broker, challenge) = begin(&mut f);
    let proof = signed_proof(&f, &challenge);
    let reply = broker
        .prove_subscription(
            proof,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo,
        )
        .unwrap();
    assert!(matches!(
        reply.body,
        NotifyBody::Subscribed {
            subscription: [8, ..],
            first_sequence: 1,
            lifetime: 30,
            ..
        }
    ));
    assert_eq!(
        nf_transport::notification::encode_body(
            &reply,
            nf_transport::notification::PROTOCOL,
            f.policy().limits
        )
        .unwrap()
        .len(),
        516
    );
}
#[test]
fn first_wrong_connection_attempt_consumes_subscription_challenge() {
    let mut f = RepoFixture::new();
    let (mut broker, challenge) = begin(&mut f);
    let proof = signed_proof(&f, &challenge);
    assert!(matches!(
        broker.prove_subscription(
            proof.clone(),
            f.client_peer,
            ConnectionId::new_unchecked(8),
            &mut f.repo
        ),
        Err(nf_transport::PeerError::Session)
    ));
    assert!(matches!(
        broker.prove_subscription(
            proof,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo
        ),
        Err(nf_transport::PeerError::Replay)
    ));
}
#[test]
fn sql_revocation_after_challenge_refuses_original_valid_proof() {
    use nf_identity::{model::DeviceRevocation, rotation::revocation_digest};
    let mut f = RepoFixture::new();
    let (mut broker, challenge) = begin(&mut f);
    let proof = signed_proof(&f, &challenge);
    let revoke = DeviceRevocation {
        scope: f.state.scope,
        issuer: f.server.public.account,
        device: f.client.public.device,
        frontier: 1,
    };
    let signature = f.server.account_key.sign(&revocation_digest(&revoke));
    f.repo.revoke_trusted(revoke, signature).unwrap();
    assert!(matches!(
        broker.prove_subscription(
            proof.clone(),
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo
        ),
        Err(nf_transport::PeerError::Unauthorized)
    ));
    assert!(matches!(
        broker.prove_subscription(
            proof,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo
        ),
        Err(nf_transport::PeerError::Replay)
    ));
}
#[test]
fn client_uses_its_original_and_verifies_real_signed_subscribed() {
    use nf_transport::notification_effects::NotifySubscriber;
    let mut f = RepoFixture::new();
    let (server, client) = f.sessions();
    let mut broker = NotifyBroker::new(server).unwrap();
    let mut subscriber = NotifySubscriber::new(client).unwrap();
    let connection = ConnectionId::new_unchecked(7);
    let begin = subscriber
        .begin_subscription(f.original.clone(), 30, &mut f.client_repo)
        .unwrap();
    let challenge = broker
        .begin_subscription(begin, f.client_peer, connection, &mut f.repo)
        .unwrap();
    let proof = subscriber
        .challenge(challenge, f.server_peer, connection, &mut f.client_repo)
        .unwrap();
    let subscribed = broker
        .prove_subscription(proof, f.client_peer, connection, &mut f.repo)
        .unwrap();
    subscriber
        .subscribed(subscribed, f.server_peer, connection, &mut f.client_repo)
        .unwrap();
}
#[test]
fn client_lifetime_starts_at_its_own_begin_even_before_subscription_proof() {
    use nf_transport::notification_effects::NotifySubscriber;
    let mut f = RepoFixture::new();
    let (server, client) = f.sessions();
    let mut broker = NotifyBroker::new(server).unwrap();
    let mut subscriber = NotifySubscriber::new(client).unwrap();
    let connection = ConnectionId::new_unchecked(7);
    let begin = subscriber
        .begin_subscription(f.original.clone(), 1, &mut f.client_repo)
        .unwrap();
    let challenge = broker
        .begin_subscription(begin, f.client_peer, connection, &mut f.repo)
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1050));
    assert!(matches!(
        subscriber.challenge(challenge, f.server_peer, connection, &mut f.client_repo),
        Err(nf_transport::PeerError::Replay)
    ));
}

#[test]
fn server_lifetime_starts_at_final_admission_not_issued_challenge() {
    let mut f = RepoFixture::new();
    let (mut broker, challenge) = begin_with_lifetime(&mut f, 2);
    let proof = signed_proof(&f, &challenge);
    std::thread::sleep(std::time::Duration::from_millis(800));
    broker
        .prove_subscription(
            proof,
            f.client_peer,
            ConnectionId::new_unchecked(7),
            &mut f.repo,
        )
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1300));
    assert!(
        broker.remaining().is_ok(),
        "server lifetime begins at final accepted Prove; client still independently bounds its own Begin"
    );
}
