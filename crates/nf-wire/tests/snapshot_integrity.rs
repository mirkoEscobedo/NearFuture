use nf_wire::{WireError, decode_chunk, decode_snapshot, generated as g};
use prost::Message;
fn from_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn snapshot() -> g::WorldSnapshot {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../protocol/vectors/nf-canon-1.json")).unwrap();
    let fixture = corpus["semantic_records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "empty-world-snapshot")
        .unwrap();
    g::WorldSnapshot {
        universe_id: Some(g::UniverseId {
            value: vec![0x44; 16],
        }),
        history_id: Some(g::HistoryId {
            value: vec![0x55; 16],
        }),
        event_seq: Some(g::EventSequence { value: 0 }),
        world_tick: Some(g::WorldTick { value: 0 }),
        ruleset_hash: Some(g::Sha256Digest {
            value: vec![0x99; 32],
        }),
        state_hash: Some(g::Sha256Digest {
            value: from_hex(fixture["sha256"].as_str().unwrap()),
        }),
        required: Some(g::RequiredSemantics::default()),
        ..Default::default()
    }
}
#[test]
fn snapshot_digest_is_the_independent_canonical_record_hash() {
    let mut value = snapshot();
    assert!(decode_snapshot(&value.encode_to_vec()).is_ok());
    value.world_tick.as_mut().unwrap().value = 1;
    assert_eq!(
        decode_snapshot(&value.encode_to_vec()),
        Err(WireError::Semantic)
    );
}
#[test]
fn chunk_quotas_and_indices_do_not_allocate_declared_totals() {
    let mut value = g::SnapshotChunk {
        transfer_id: Some(g::OperationId { value: vec![1; 16] }),
        chunk_index: 0,
        chunk_count: 1,
        total_bytes: 1,
        snapshot_digest: Some(g::Sha256Digest { value: vec![0; 32] }),
        data: vec![3],
    };
    assert!(decode_chunk(&value.encode_to_vec()).is_ok());
    value.total_bytes = 67_108_865;
    assert_eq!(decode_chunk(&value.encode_to_vec()), Err(WireError::Limit));
    value.total_bytes = 1;
    value.chunk_index = 1;
    assert_eq!(
        decode_chunk(&value.encode_to_vec()),
        Err(WireError::Semantic)
    );
    value.chunk_index = 0;
    value.data.clear();
    assert_eq!(
        decode_chunk(&value.encode_to_vec()),
        Err(WireError::Semantic)
    );
}
