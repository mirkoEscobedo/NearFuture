#[path = "receipt_support/identities.rs"]
mod identities;
use identities::Pair;
use nf_transport::{PeerError, receipt::ReceiptBody};
#[test]
fn first_failed_authentication_attempt_consumes_the_stage() {
    let p = Pair::new();
    let (mut client, mut server) = p.sessions();
    let good = server.send(&p.state, &p.server_key).unwrap();
    let mut bad = good.clone();
    let ReceiptBody::ServerHello { proof, .. } = &mut bad.body else {
        panic!("server hello")
    };
    proof.signature[0] ^= 1;
    let peer = p.handshake.context().server_peer;
    assert_eq!(
        client.receive(bad, peer, p.connection, &p.state),
        Err(PeerError::Unauthorized)
    );
    assert_eq!(
        client.receive(good, peer, p.connection, &p.state),
        Err(PeerError::Replay),
        "a valid retry cannot revive consumed proof state"
    );
    assert!(!client.active());
    let (mut client, mut server) = p.sessions();
    client
        .receive(
            server.send(&p.state, &p.server_key).unwrap(),
            peer,
            p.connection,
            &p.state,
        )
        .unwrap();
    server
        .receive(
            client.send(&p.state, &p.client_key).unwrap(),
            p.handshake.context().client_peer,
            p.connection,
            &p.state,
        )
        .unwrap();
    client
        .receive(
            server.send(&p.state, &p.server_key).unwrap(),
            peer,
            p.connection,
            &p.state,
        )
        .unwrap();
    assert!(client.active() && server.active());
}
#[test]
fn fresh_current_revocation_fences_both_endpoints_after_activation() {
    use nf_identity::{model::DeviceRevocation, rotation::revocation_digest};
    let p = Pair::new();
    let (mut client, mut server) = p.sessions();
    let c = p.handshake.context();
    client
        .receive(
            server.send(&p.state, &p.server_key).unwrap(),
            c.server_peer,
            p.connection,
            &p.state,
        )
        .unwrap();
    server
        .receive(
            client.send(&p.state, &p.client_key).unwrap(),
            c.client_peer,
            p.connection,
            &p.state,
        )
        .unwrap();
    client
        .receive(
            server.send(&p.state, &p.server_key).unwrap(),
            c.server_peer,
            p.connection,
            &p.state,
        )
        .unwrap();
    let change = DeviceRevocation {
        scope: p.state.scope,
        issuer: p.server.account,
        device: p.server.device,
        frontier: p.state.revision,
    };
    let revoked = p
        .state
        .revoke_device(
            &change,
            &p.server_account_key.sign(&revocation_digest(&change)),
        )
        .unwrap();
    let record = nf_transport::receipt::decode_body(
        &super_record(&p),
        nf_transport::records::PeerLimits::default(),
    )
    .unwrap();
    assert_eq!(
        client.admit_record(&record, c.server_peer, p.connection, &revoked),
        Err(PeerError::Unauthorized)
    );
    assert_eq!(
        server.admit_record(&record, c.client_peer, p.connection, &revoked),
        Err(PeerError::Unauthorized)
    );
    assert!(!client.active() && !server.active());
}
fn super_record(p: &Pair) -> Vec<u8> {
    use nf_contract::identity::{OperationId, RequestId};
    use nf_transport::receipt::*;
    let r = ReceiptRecord {
        context: p.handshake.context().context,
        body: ReceiptBody::Begin {
            target: ReceiptTarget {
                request: RequestId::from_bytes([1; 16]),
                operation: OperationId::from_bytes([2; 16]),
                binding: [3; 32],
            },
            nonce: [4; 32],
            minimum: SourceMinima {
                event: nf_contract::identity::EventSeq(0),
                store_revision: 0,
                membership_revision: p.state.revision,
            },
        },
    };
    encode_body(&r, nf_transport::records::PeerLimits::default()).unwrap()
}
#[test]
fn purpose_actual_connection_and_current_player_are_mandatory() {
    use nf_identity::{model::Roles, signing::device_digest};
    let p = Pair::new();
    let (mut client, mut server) = p.sessions();
    let c = p.handshake.context();
    let mut wrong = server.send(&p.state, &p.server_key).unwrap();
    let ReceiptBody::ServerHello { proof, .. } = &mut wrong.body else {
        panic!("server hello")
    };
    proof.challenge = c.challenge(1, p.state.revision).unwrap();
    proof.signature = p.server_key.sign(&device_digest(proof).unwrap());
    assert_eq!(
        client.receive(wrong, c.server_peer, p.connection, &p.state),
        Err(PeerError::Unauthorized),
        "maintained valid signature in control1 domain is not control2 proof"
    );
    let (mut client, mut server) = p.sessions();
    assert_eq!(
        client.receive(
            server.send(&p.state, &p.server_key).unwrap(),
            c.server_peer,
            libp2p::swarm::ConnectionId::new_unchecked(2),
            &p.state
        ),
        Err(PeerError::Session)
    );
    assert!(!client.active());
    let worker = Pair::with_roles(Roles::WORKER);
    let wc = worker.handshake.context();
    let binding = nf_transport::receipt::ReceiptSessionBinding {
        endpoint: nf_transport::receipt::ReceiptEndpoint::Client,
        local: worker.client.clone(),
        connection: worker.connection,
        server_pin: nf_transport::auth::ServerPin {
            peer: wc.server_peer,
            account: wc.server_account,
            device: wc.server_device,
            minimum_membership: worker.state.revision,
        },
        minimum_membership: worker.state.revision,
    };
    assert!(matches!(
        nf_transport::receipt::ReceiptSession::new(
            worker.handshake.clone(),
            binding,
            &worker.state
        ),
        Err(PeerError::Unauthorized)
    ));
}
