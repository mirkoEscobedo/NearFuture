#![allow(dead_code)] // Shared signed/SQLite scenario helpers are compiled by separate tests.
use super::community::Community;
use nf_identity::model::{MembershipRepository, MembershipState};
use nf_kernel::{World, WorldSpec};
use nf_store::Store;
use nf_transport::{
    auth::ServerPin,
    query::{QueryClient, QueryServer},
    records::{Lane, PeerLimits},
    session::{ClientSession, ServerSession, SessionPolicy},
};
pub fn store(c: &Community, p: std::path::PathBuf) -> Store {
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
pub fn pair(
    c: &Community,
    root: &std::path::Path,
) -> (
    QueryClient,
    QueryServer,
    libp2p::PeerId,
    libp2p::PeerId,
    libp2p::swarm::ConnectionId,
) {
    with_server(c, root, store(c, root.join("server.sqlite")))
}
pub fn with_server(
    c: &Community,
    root: &std::path::Path,
    ss: Store,
) -> (
    QueryClient,
    QueryServer,
    libp2p::PeerId,
    libp2p::PeerId,
    libp2p::swarm::ConnectionId,
) {
    let cs = store(c, root.join("client.sqlite"));
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
    (
        QueryClient::new(client, cs).unwrap(),
        QueryServer::new(server, ss).unwrap(),
        cp,
        sp,
        id,
    )
}
