use super::community::Community;
use nf_transport::{
    auth::ServerPin,
    bulk::{BulkClient, BulkServer},
    records::{Lane, PeerLimits},
    session::{ClientSession, ServerSession, SessionPolicy},
};
pub fn pair(
    c: &Community,
) -> (
    BulkClient,
    BulkServer,
    libp2p::PeerId,
    libp2p::PeerId,
    libp2p::swarm::ConnectionId,
) {
    let cp = c.client_noise.public().to_peer_id();
    let sp = c.server_noise.public().to_peer_id();
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
        ClientSession::begin(c.client.clone(), cp, pin, Lane::Bulk, policy).unwrap();
    let (mut server, reply) = ServerSession::begin(
        hello,
        cp,
        id,
        c.server.clone(),
        sp,
        Lane::Bulk,
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
        BulkClient::new(client, &c.state).unwrap(),
        BulkServer::new(server, &c.state).unwrap(),
        cp,
        sp,
        id,
    )
}
