#[path = "support/community.rs"]
mod community;
use community::Community;
use nf_identity::model::Roles;
use nf_transport::{
    PeerError,
    auth::ServerPin,
    records::{Lane, PeerLimits},
    session::{ClientSession, ServerSession, SessionPolicy},
};
fn policy(c: &Community) -> SessionPolicy {
    SessionPolicy {
        scope: c.state.scope,
        ruleset: [4; 32],
        content: [5; 32],
        limits: PeerLimits::default(),
        minimum_membership: 1,
    }
}
#[test]
fn application_session_requires_current_admitted_device_peer_player_and_both_proof_stages() {
    let c = Community::new(Roles::PLAYER);
    let sp = c.server_noise.public().to_peer_id();
    let cp = c.client_noise.public().to_peer_id();
    let id = libp2p::swarm::ConnectionId::new_unchecked(7);
    let pin = ServerPin {
        peer: sp,
        account: c.server.account,
        device: c.server.device,
        minimum_membership: 1,
    };
    let (mut client, hello) =
        ClientSession::begin(c.client.clone(), cp, pin, Lane::Control, policy(&c)).unwrap();
    let (mut server, reply) = ServerSession::begin(
        hello,
        cp,
        id,
        c.server.clone(),
        sp,
        Lane::Control,
        policy(&c),
        &c.state,
        &c.server_device,
    )
    .unwrap();
    let proof = client
        .server_hello(reply, sp, id, &c.state, &c.client_device)
        .unwrap();
    assert!(!client.active());
    let finished = server
        .client_proof(proof.clone(), cp, id, &c.state, &c.server_device)
        .unwrap();
    client.finished(finished, sp, id, &c.state).unwrap();
    assert!(client.active());
    assert!(server.active());
    assert_eq!(
        server.client_proof(proof, cp, id, &c.state, &c.server_device),
        Err(PeerError::Replay)
    );
    let c = Community::new(Roles::WORKER);
    let sp = c.server_noise.public().to_peer_id();
    let cp = c.client_noise.public().to_peer_id();
    let pin = ServerPin {
        peer: sp,
        account: c.server.account,
        device: c.server.device,
        minimum_membership: 1,
    };
    let (mut client, hello) =
        ClientSession::begin(c.client.clone(), cp, pin, Lane::Control, policy(&c)).unwrap();
    let (_, reply) = ServerSession::begin(
        hello,
        cp,
        id,
        c.server.clone(),
        sp,
        Lane::Control,
        policy(&c),
        &c.state,
        &c.server_device,
    )
    .unwrap();
    assert_eq!(
        client.server_hello(reply, sp, id, &c.state, &c.client_device),
        Err(PeerError::Unauthorized)
    );
    assert!(!client.active());
}
