#[path = "support/community.rs"]
mod community;
#[path = "support/scratch.rs"]
mod scratch;
use community::Community;
use nf_contract::identity::RequestId;
use nf_identity::model::{MembershipRepository, MembershipState, Roles};
use nf_kernel::{World, WorldSpec};
use nf_store::Store;
use nf_transport::{
    PeerError,
    auth::ServerPin,
    query::{QueryClient, QueryResult, QueryServer},
    records::{Lane, PeerLimits, RetainedPhase},
    session::{ClientSession, ServerSession, SessionPolicy},
};
use std::time::{Duration, Instant};
fn store(c: &Community, p: std::path::PathBuf) -> Store {
    let world = World::new(WorldSpec::empty(
        c.state.scope.universe,
        c.state.scope.history,
        [3; 32],
        [4; 32],
    ))
    .unwrap();
    let mut s = Store::create(p, &world).unwrap();
    let founder = MembershipState::bootstrap(c.state.scope, &c.server).unwrap();
    s.commit_membership(None, &founder).unwrap();
    s.commit_membership(Some(0), &c.state).unwrap();
    s
}
#[test]
fn protected_unknown_status_consumes_challenge_and_rechecks_actual_durable_membership() {
    let c = Community::new(Roles::PLAYER);
    let scratch = scratch::Scratch::new();
    let ss = store(&c, scratch.0.join("server.sqlite"));
    let cs = store(&c, scratch.0.join("client.sqlite"));
    let sp = c.server_noise.public().to_peer_id();
    let cp = c.client_noise.public().to_peer_id();
    let id = libp2p::swarm::ConnectionId::new_unchecked(7);
    let policy = SessionPolicy {
        scope: c.state.scope,
        ruleset: [4; 32],
        content: [5; 32],
        limits: PeerLimits::default(),
        minimum_membership: 1,
    };
    let pin = ServerPin {
        peer: sp,
        account: c.server.account,
        device: c.server.device,
        minimum_membership: 1,
    };
    let (mut client, hello) =
        ClientSession::begin(c.client.clone(), cp, pin, Lane::Control, policy).unwrap();
    let (mut server, reply) = ServerSession::begin(
        hello,
        cp,
        id,
        c.server.clone(),
        sp,
        Lane::Control,
        policy,
        &c.state,
        &c.server_device,
    )
    .unwrap();
    let proof = client
        .server_hello(reply, sp, id, &c.state, &c.client_device)
        .unwrap();
    let finished = server
        .client_proof(proof, cp, id, &c.state, &c.server_device)
        .unwrap();
    client.finished(finished, sp, id, &c.state).unwrap();
    let mut client = QueryClient::new(client, cs).unwrap();
    let mut server = QueryServer::new(server, ss).unwrap();
    let now = Instant::now();
    let request = RequestId::from_bytes([9; 16]);
    let begin = client.begin(request, now).unwrap();
    let challenge = server.begin(begin, cp, id, now).unwrap();
    let proof = client
        .challenge(challenge, sp, id, &c.client_device, now)
        .unwrap();
    let response = server
        .prove(proof.clone(), cp, id, &c.server_device, now)
        .unwrap();
    assert!(matches!(
        client.reply(response, sp, id, now).unwrap(),
        QueryResult::Status(RetainedPhase::UnknownRequest)
    ));
    assert_eq!(
        server.prove(proof, cp, id, &c.server_device, now),
        Err(PeerError::Replay)
    );
    let mut begin = client.begin(request, now).unwrap();
    if let nf_transport::records::PeerBody::BeginQuery {
        minimum_membership, ..
    } = &mut begin.body
    {
        *minimum_membership = 0;
    }
    let challenge = server.begin(begin, cp, id, now).unwrap();
    assert_eq!(
        client.challenge(challenge, sp, id, &c.client_device, now),
        Err(PeerError::Unauthorized)
    );
    let later = now + Duration::from_secs(6);
    let begin = client.begin(request, later).unwrap();
    let challenge = server.begin(begin, cp, id, later).unwrap();
    let proof = client
        .challenge(challenge, sp, id, &c.client_device, later)
        .unwrap();
    assert_eq!(
        server.prove(
            proof,
            cp,
            id,
            &c.server_device,
            later + Duration::from_secs(6)
        ),
        Err(PeerError::Replay)
    );
}
