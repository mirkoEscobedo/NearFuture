use nf_wire::{WireError, decode_control, generated as g};
use prost::Message;
fn intent() -> g::ControlEnvelope {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../protocol/vectors/wire-v1.json")).unwrap();
    let case = corpus["positive"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "known-required-contract-probe")
        .unwrap();
    let hex = case["wire_hex"].as_str().unwrap();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    decode_control(&bytes).unwrap()
}
fn version(byte: u8, revision: u64) -> g::AggregateVersion {
    g::AggregateVersion {
        aggregate_id: Some(g::AggregateId {
            value: vec![byte; 16],
        }),
        revision: Some(g::AggregateRevision { value: revision }),
    }
}
#[test]
fn wire_revision_maps_reject_duplicates_and_unsorted_ids() {
    let mut envelope = intent();
    let Some(g::control_envelope::Body::Intent(value)) = &mut envelope.body else {
        panic!("fixture must be an intent")
    };
    value.expected_revisions = vec![version(2, 0), version(3, u64::MAX)];
    assert!(decode_control(&envelope.encode_to_vec()).is_ok());
    let Some(g::control_envelope::Body::Intent(value)) = &mut envelope.body else {
        unreachable!()
    };
    value.expected_revisions = vec![version(2, 0), version(2, 1)];
    assert_eq!(
        decode_control(&envelope.encode_to_vec()),
        Err(WireError::Semantic)
    );
    let Some(g::control_envelope::Body::Intent(value)) = &mut envelope.body else {
        unreachable!()
    };
    value.expected_revisions = vec![version(3, 0), version(2, 1)];
    assert_eq!(
        decode_control(&envelope.encode_to_vec()),
        Err(WireError::Semantic)
    );
}
#[test]
fn pending_status_cannot_claim_a_committed_success_frontier() {
    let mut envelope = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 1 }),
        required: Some(g::RequiredSemantics::default()),
        transport: None,
        body: Some(g::control_envelope::Body::OperationStatus(
            g::OperationStatus {
                request_id: Some(g::RequestId { value: vec![1; 16] }),
                operation_id: Some(g::OperationId { value: vec![2; 16] }),
                history_id: Some(g::HistoryId { value: vec![3; 16] }),
                request_binding_digest: Some(g::Sha256Digest { value: vec![4; 32] }),
                phase: g::OperationPhase::Pending as i32,
                outcome: None,
                committed_event_seq: None,
            },
        )),
    };
    assert!(decode_control(&envelope.encode_to_vec()).is_ok());
    let Some(g::control_envelope::Body::OperationStatus(value)) = &mut envelope.body else {
        unreachable!()
    };
    value.committed_event_seq = Some(g::EventSequence { value: 0 });
    assert_eq!(
        decode_control(&envelope.encode_to_vec()),
        Err(WireError::Semantic)
    );
}
#[test]
fn exact_signature_shape_is_separate_from_authorization() {
    let mut envelope = intent();
    let Some(g::control_envelope::Body::Intent(value)) = &mut envelope.body else {
        unreachable!()
    };
    value.signature = vec![0; 64];
    // Shape admission grants no authority and does not verify a principal/key.
    assert!(decode_control(&envelope.encode_to_vec()).is_ok());
    let Some(g::control_envelope::Body::Intent(value)) = &mut envelope.body else {
        unreachable!()
    };
    value.signature.pop();
    assert_eq!(
        decode_control(&envelope.encode_to_vec()),
        Err(WireError::Semantic)
    );
}
