use nf_ipc::*;
use nf_wire::generated as g;
use prost::Message;
fn required() -> g::RequiredSemantics {
    g::RequiredSemantics {
        capability_ids: vec![1],
        schema_ids: vec![1],
    }
}
fn session() -> AuthenticatedSession {
    use nf_contract::identity::{AccountId, DeviceId};
    let binding = AuthBinding {
        config: SessionConfig {
            universe: [1; 16],
            history: [2; 16],
            runtime_session: 9,
            ruleset: [3; 32],
            content_policy: [4; 32],
            limits: default_limits(),
        },
        principal: LocalPrincipal {
            account: AccountId::from_bytes([5; 16]),
            device: DeviceId::from_bytes([6; 16]),
        },
        role: EndpointRole::Control,
        port: 12345,
    };
    let auth = Authenticator::new(binding.clone(), SecretToken::from_bytes([7; 32])).unwrap();
    let client = ClientHello::start(
        binding.clone(),
        SecretToken::from_bytes([7; 32]),
        binding.config.limits,
    )
    .unwrap();
    let challenge = auth.begin(client.hello()).unwrap();
    let proof = client.respond(challenge.challenge()).unwrap();
    let (_, accepted) = auth.finish(challenge, proof.proof()).unwrap();
    proof.finish(&accepted).unwrap()
}
#[test]
fn trusted_principal_and_explicit_lifecycle_fence_gate_queries() {
    use nf_contract::identity::{AccountId, DeviceId};
    use nf_ipc::{LocalPrincipal, SessionFence};
    let session = session();
    let principal = LocalPrincipal {
        account: AccountId::from_bytes([5; 16]),
        device: DeviceId::from_bytes([6; 16]),
    };
    let mut fence = SessionFence::new(9).unwrap();
    let mut query = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 9 }),
        required: Some(required()),
        body: Some(g::control_envelope::Body::QueryOperation(
            g::QueryOperation {
                request_id: Some(g::RequestId { value: vec![8; 16] }),
                principal: Some(g::Principal {
                    account_id: Some(g::AccountId { value: vec![5; 16] }),
                    device_id: Some(g::DeviceId { value: vec![6; 16] }),
                }),
                universe_id: Some(g::UniverseId { value: vec![1; 16] }),
                history_id: Some(g::HistoryId { value: vec![2; 16] }),
            },
        )),
        transport: None,
    };
    assert!(
        session
            .admit_client(&query.encode_to_vec(), principal, &fence)
            .is_ok()
    );
    if let Some(g::control_envelope::Body::QueryOperation(q)) = &mut query.body {
        q.principal
            .as_mut()
            .unwrap()
            .device_id
            .as_mut()
            .unwrap()
            .value = vec![10; 16];
    }
    assert!(matches!(
        session.admit_client(&query.encode_to_vec(), principal, &fence),
        Err(IpcError::Unauthorized)
    ));
    fence.invalidate();
    assert!(matches!(
        session.admit_client(&query.encode_to_vec(), principal, &fence),
        Err(IpcError::ReadOnly)
    ));
}
#[test]
fn authenticated_client_rejects_old_session_wrong_history_and_wrong_request_status() {
    use nf_contract::identity::RequestId;
    let session = session();
    let mut fence = SessionFence::new(9).unwrap();
    let request = RequestId::from_bytes([8; 16]);
    let mut envelope = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 9 }),
        required: Some(required()),
        body: Some(g::control_envelope::Body::OperationStatus(
            g::OperationStatus {
                request_id: Some(g::RequestId { value: vec![8; 16] }),
                operation_id: Some(g::OperationId { value: vec![8; 16] }),
                history_id: Some(g::HistoryId { value: vec![2; 16] }),
                request_binding_digest: Some(g::Sha256Digest { value: vec![7; 32] }),
                phase: g::OperationPhase::Pending as i32,
                outcome: None,
                committed_event_seq: None,
            },
        )),
        transport: None,
    };
    assert!(matches!(
        session.admit_response(&envelope.encode_to_vec(), request, &fence),
        Ok(AdmittedServer::Status(_))
    ));
    envelope.runtime_session.as_mut().unwrap().value = 10;
    assert!(matches!(
        session.admit_response(&envelope.encode_to_vec(), request, &fence),
        Err(IpcError::SessionMismatch)
    ));
    envelope.runtime_session.as_mut().unwrap().value = 9;
    let Some(g::control_envelope::Body::OperationStatus(s)) = &mut envelope.body else {
        panic!("status")
    };
    s.history_id.as_mut().unwrap().value = vec![99; 16];
    assert!(matches!(
        session.admit_response(&envelope.encode_to_vec(), request, &fence),
        Err(IpcError::HistoryMismatch)
    ));
    let Some(g::control_envelope::Body::OperationStatus(s)) = &mut envelope.body else {
        panic!("status")
    };
    s.history_id.as_mut().unwrap().value = vec![2; 16];
    s.request_id.as_mut().unwrap().value = vec![99; 16];
    assert!(matches!(
        session.admit_response(&envelope.encode_to_vec(), request, &fence),
        Err(IpcError::Malformed)
    ));
    fence.invalidate();
    assert!(matches!(
        session.admit_response(&envelope.encode_to_vec(), request, &fence),
        Err(IpcError::ReadOnly)
    ));
}
