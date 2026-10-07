use nf_contract::{
    canonical::{Limits, Profile, encode_profile},
    identity::{AccountId, DeviceId},
};
use nf_ipc::*;
use nf_wire::generated as g;
use prost::Message;
use std::{
    net::TcpStream,
    time::{Duration, Instant},
};
struct OversizeProbe;
impl QueryPort for OversizeProbe {
    fn query(&mut self, q: g::QueryOperation) -> Result<g::OperationStatus, IpcError> {
        Ok(g::OperationStatus {
            request_id: q.request_id,
            operation_id: Some(g::OperationId { value: vec![8; 16] }),
            history_id: q.history_id,
            request_binding_digest: Some(g::Sha256Digest { value: vec![9; 32] }),
            phase: g::OperationPhase::Succeeded as i32,
            outcome: Some(g::operation_status::Outcome::Success(g::SchemaPayload {
                schema_id: 1,
                canonical_body: encode_profile(
                    &Profile {
                        optional_bytes: Some(vec![7; 256]),
                        ..Default::default()
                    },
                    Limits::default(),
                )
                .unwrap(),
            })),
            committed_event_seq: Some(g::EventSequence { value: 1 }),
        })
    }
}
fn receive<P: QueryPort>(node: &mut NodeServer<P>, peer: &mut FramePump) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        node.poll(Instant::now()).unwrap();
        if let Some(body) = peer.poll(4096, 4096).unwrap() {
            return body;
        }
        assert!(Instant::now() < deadline);
    }
}
#[test]
fn trusted_query_response_must_also_obey_authenticated_negotiated_field_budget() {
    let mut limits = default_limits();
    limits.chunk_bytes = 64;
    let config = SessionConfig {
        universe: [1; 16],
        history: [2; 16],
        runtime_session: 9,
        ruleset: [3; 32],
        content_policy: [4; 32],
        limits,
    };
    let principal = LocalPrincipal {
        account: AccountId::from_bytes([5; 16]),
        device: DeviceId::from_bytes([6; 16]),
    };
    let mut node = NodeServer::bind(config.clone(), principal, OversizeProbe).unwrap();
    let binding = AuthBinding {
        config,
        principal,
        role: EndpointRole::Control,
        port: node.address().port(),
    };
    node.install_authenticator(
        Authenticator::new(binding.clone(), SecretToken::from_bytes([7; 32])).unwrap(),
    )
    .unwrap();
    let mut peer = FramePump::new(TcpStream::connect(node.address()).unwrap(), 4096).unwrap();
    let hello = ClientHello::start(binding, SecretToken::from_bytes([7; 32]), limits).unwrap();
    peer.send(hello.hello()).unwrap();
    let proof = hello.respond(&receive(&mut node, &mut peer)).unwrap();
    peer.send(proof.proof()).unwrap();
    let session = proof.finish(&receive(&mut node, &mut peer)).unwrap();
    peer.activate(&session).unwrap();
    let query = g::QueryOperation {
        request_id: Some(g::RequestId { value: vec![8; 16] }),
        principal: Some(g::Principal {
            account_id: Some(g::AccountId { value: vec![5; 16] }),
            device_id: Some(g::DeviceId { value: vec![6; 16] }),
        }),
        universe_id: Some(g::UniverseId { value: vec![1; 16] }),
        history_id: Some(g::HistoryId { value: vec![2; 16] }),
    };
    let request = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 9 }),
        required: Some(g::RequiredSemantics {
            capability_ids: vec![1],
            schema_ids: vec![1],
        }),
        body: Some(g::control_envelope::Body::QueryOperation(query)),
        transport: None,
    }
    .encode_to_vec();
    peer.send(&request).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        node.poll(Instant::now()).unwrap();
        match peer.poll(4096, 4096) {
            Err(IpcError::Disconnected) => break,
            Ok(None) => {}
            other => panic!("oversized probe must never be emitted: {other:?}"),
        };
        assert!(Instant::now() < deadline);
    }
    assert_eq!(node.connection_count(), 0);
}
