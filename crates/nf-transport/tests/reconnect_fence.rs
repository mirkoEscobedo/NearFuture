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
#[test]
fn fresh_session_and_connection_reject_old_valid_device_proof_before_admitting_queued_work() {
    let c = Community::new(Roles::PLAYER);
    let sp = c.server_noise.public().to_peer_id();
    let cp = c.client_noise.public().to_peer_id();
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
    let id1 = libp2p::swarm::ConnectionId::new_unchecked(7);
    let id2 = libp2p::swarm::ConnectionId::new_unchecked(8);
    let (mut old, hello) =
        ClientSession::begin(c.client.clone(), cp, pin, Lane::Control, policy).unwrap();
    let (_, reply) = ServerSession::begin(
        hello,
        cp,
        id1,
        c.server.clone(),
        sp,
        Lane::Control,
        policy,
        &c.state,
        &c.server_device,
    )
    .unwrap();
    let old_proof = old
        .server_hello(reply, sp, id1, &c.state, &c.client_device)
        .unwrap();
    for splice in [false, true] {
        let (_, hello) =
            ClientSession::begin(c.client.clone(), cp, pin, Lane::Control, policy).unwrap();
        let (mut new, reply) = ServerSession::begin(
            hello,
            cp,
            id2,
            c.server.clone(),
            sp,
            Lane::Control,
            policy,
            &c.state,
            &c.server_device,
        )
        .unwrap();
        let mut replay = old_proof.clone();
        if splice {
            replay.context = reply.context;
        }
        let error = new
            .client_proof(replay.clone(), cp, id2, &c.state, &c.server_device)
            .unwrap_err();
        assert_eq!(
            error,
            if splice {
                PeerError::Unauthorized
            } else {
                PeerError::Session
            }
        );
        assert!(!new.active());
        assert_eq!(
            new.client_proof(replay, cp, id2, &c.state, &c.server_device),
            Err(PeerError::Replay)
        );
    }
}
