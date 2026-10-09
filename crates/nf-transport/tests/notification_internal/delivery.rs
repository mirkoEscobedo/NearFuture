use super::support as notification_effect_support;
use libp2p::swarm::ConnectionId;
use nf_transport::{
    PeerError,
    notification_effects::{NotifyClientHandshake, NotifyServerHandshake},
};
use notification_effect_support::repo::RepoFixture;
#[test]
fn handshake_outputs_get_single_use_current_repo_delivery_custody() {
    let mut f = RepoFixture::new();
    let connection = ConnectionId::new_unchecked(7);
    let (mut client, hello) = NotifyClientHandshake::begin(
        f.client.public.clone(),
        f.client_peer,
        f.original.source(),
        f.policy(),
    )
    .unwrap();
    let hello = client
        .prepare_output(hello, f.server_peer, connection, &mut f.client_repo)
        .unwrap();
    let wire_hello = hello.record().clone();
    let (mut server, reply) = NotifyServerHandshake::begin(
        wire_hello,
        f.client_peer,
        connection,
        f.server.public.clone(),
        f.server_peer,
        f.policy(),
        &f.state,
        &f.server.device_key,
    )
    .unwrap();
    let server_reply = server
        .prepare_output(reply.clone(), f.client_peer, connection, &mut f.repo)
        .unwrap();
    assert!(matches!(
        server.prepare_output(reply.clone(), f.client_peer, connection, &mut f.repo),
        Err(PeerError::Replay)
    ));
    let proof = client
        .server_hello(
            reply,
            f.server_peer,
            connection,
            &f.state,
            &f.client.device_key,
        )
        .unwrap();
    client
        .complete_emission(hello, f.server_peer, connection, &mut f.client_repo)
        .unwrap();
    server
        .complete_emission(server_reply, f.client_peer, connection, &mut f.repo)
        .unwrap();
    let proof = client
        .prepare_output(proof, f.server_peer, connection, &mut f.client_repo)
        .unwrap();
    let (mut server_active, finished) = server
        .client_proof(
            proof.record().clone(),
            f.client_peer,
            connection,
            &f.state,
            &f.server.device_key,
        )
        .unwrap();
    client
        .complete_emission(proof, f.server_peer, connection, &mut f.client_repo)
        .unwrap();
    let finished = server_active
        .prepare_output(finished, f.client_peer, connection, &mut f.repo)
        .unwrap();
    let client_active = client
        .finished(
            finished.record().clone(),
            f.server_peer,
            connection,
            &f.state,
        )
        .unwrap();
    server_active
        .complete_emission(finished, f.client_peer, connection, &mut f.repo)
        .unwrap();
    assert!(client_active.active() && server_active.active());
}
