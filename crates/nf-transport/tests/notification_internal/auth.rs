use super::support as notification_effect_support;
use libp2p::swarm::ConnectionId;
use nf_identity::model::Roles;
use nf_transport::notification::{NotifyBody, PROTOCOL, encode_body};
use nf_transport::notification_effects::{NotifyClientHandshake, NotifyServerHandshake};
use notification_effect_support::Fixture;
#[test]
fn generated_sql_membership_drives_real_four_stage_notification_handshake() {
    let mut f = Fixture::new(Roles::PLAYER);
    let connection = ConnectionId::new_unchecked(7);
    let (mut client, hello) =
        NotifyClientHandshake::begin(f.client.clone(), f.client_peer, f.pin(), f.policy()).unwrap();
    let current = f.current();
    let (mut server, reply) = NotifyServerHandshake::begin(
        hello,
        f.client_peer,
        connection,
        f.server.clone(),
        f.server_peer,
        f.policy(),
        &current,
        &f.server_device,
    )
    .unwrap();
    assert!(matches!(reply.body, NotifyBody::ServerHello { .. }));
    assert_eq!(
        encode_body(&reply, PROTOCOL, f.policy().limits)
            .unwrap()
            .len(),
        493
    );
    let current = f.current();
    let proof = client
        .server_hello(reply, f.server_peer, connection, &current, &f.client_device)
        .unwrap();
    let current = f.current();
    let (server_active, finished) = server
        .client_proof(proof, f.client_peer, connection, &current, &f.server_device)
        .unwrap();
    let current = f.current();
    let client_active = client
        .finished(finished, f.server_peer, connection, &current)
        .unwrap();
    assert!(server_active.active() && client_active.active());
    assert_eq!(server_active.context(), client_active.context());
}

#[test]
fn first_wrong_connection_proof_attempt_consumes_server_handshake() {
    let mut f = Fixture::new(Roles::PLAYER);
    let connection = ConnectionId::new_unchecked(7);
    let (mut client, hello) =
        NotifyClientHandshake::begin(f.client.clone(), f.client_peer, f.pin(), f.policy()).unwrap();
    let current = f.current();
    let (mut server, reply) = NotifyServerHandshake::begin(
        hello,
        f.client_peer,
        connection,
        f.server.clone(),
        f.server_peer,
        f.policy(),
        &current,
        &f.server_device,
    )
    .unwrap();
    let current = f.current();
    let proof = client
        .server_hello(reply, f.server_peer, connection, &current, &f.client_device)
        .unwrap();
    let current = f.current();
    assert!(matches!(
        server.client_proof(
            proof.clone(),
            f.client_peer,
            ConnectionId::new_unchecked(8),
            &current,
            &f.server_device
        ),
        Err(nf_transport::PeerError::Session)
    ));
    let current = f.current();
    assert!(matches!(
        server.client_proof(proof, f.client_peer, connection, &current, &f.server_device),
        Err(nf_transport::PeerError::Replay)
    ));
}

#[test]
fn real_signed_sql_revocation_fences_reply_without_reusing_matching_numeric_shape() {
    use nf_identity::{
        model::DeviceRevocation, persistence::revoke_persisted, rotation::revocation_digest,
    };
    let mut f = Fixture::new(Roles::PLAYER);
    let connection = ConnectionId::new_unchecked(7);
    let (mut client, hello) =
        NotifyClientHandshake::begin(f.client.clone(), f.client_peer, f.pin(), f.policy()).unwrap();
    let current = f.current();
    let (_, reply) = NotifyServerHandshake::begin(
        hello,
        f.client_peer,
        connection,
        f.server.clone(),
        f.server_peer,
        f.policy(),
        &current,
        &f.server_device,
    )
    .unwrap();
    let change = DeviceRevocation {
        scope: f.scope,
        issuer: f.server.account,
        device: f.client.device,
        frontier: current.revision,
    };
    let signature = f.server_account.sign(&revocation_digest(&change));
    revoke_persisted(&mut f.store, f.scope, &change, &signature).unwrap();
    let current = f.current();
    assert!(matches!(
        client.server_hello(
            reply.clone(),
            f.server_peer,
            connection,
            &current,
            &f.client_device
        ),
        Err(nf_transport::PeerError::Unauthorized)
    ));
    assert!(matches!(
        client.server_hello(reply, f.server_peer, connection, &current, &f.client_device),
        Err(nf_transport::PeerError::Replay)
    ));
}

#[test]
fn non_economic_member_cannot_create_active_notification_session() {
    let mut f = Fixture::new(Roles::WORKER);
    let connection = ConnectionId::new_unchecked(7);
    let (_, hello) =
        NotifyClientHandshake::begin(f.client.clone(), f.client_peer, f.pin(), f.policy()).unwrap();
    let current = f.current();
    assert!(matches!(
        NotifyServerHandshake::begin(
            hello,
            f.client_peer,
            connection,
            f.server.clone(),
            f.server_peer,
            f.policy(),
            &current,
            &f.server_device
        ),
        Err(nf_transport::PeerError::Unauthorized)
    ));
}
