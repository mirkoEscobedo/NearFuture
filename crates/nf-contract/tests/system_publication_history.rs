use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_publication::{
        PublicationHistory, PublicationKind, PublicationLimits, PublicationOutcome,
        PublicationProfile, PublicationRecord,
    },
    system_topology::{Limits, Topology, decode, digest, encode, validate},
};

const A1_SHA256: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const A2_SHA256: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
const A2_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440200000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const HISTORY_LIMITS: PublicationLimits = PublicationLimits {
    keys: 1,
    records: 3,
    canonical_bytes: 398,
};

fn literal_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_input(value: &Topology, original: &Topology, bytes: &[u8], sha256: &str) {
    let limits = Limits::default();
    assert_eq!(value, original);
    assert_eq!(validate(value, limits), Ok(()));
    assert_eq!(encode(value, limits).unwrap().as_slice(), bytes);
    assert_eq!(
        digest(value, limits).unwrap().as_slice(),
        literal_bytes(sha256).as_slice()
    );
}

fn assert_record(
    record: &PublicationRecord,
    kind: PublicationKind,
    revision: u64,
    referenced_revision: u64,
    sha256: &str,
    body: Option<&[u8]>,
) {
    assert_eq!(record.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(record.history(), HistoryId::from_bytes([0x22; 16]));
    assert_eq!(record.owner(), AccountId::from_bytes([0x33; 16]));
    assert_eq!(record.system(), EntityId::from_bytes([0x44; 16]));
    assert_eq!(record.profile(), PublicationProfile::SelectedTopologyV1);
    assert_eq!(record.kind(), kind);
    assert_eq!(record.revision(), revision);
    assert_eq!(record.referenced_publication_revision(), referenced_revision);
    assert_eq!(
        record.referenced_topology_sha256().as_slice(),
        literal_bytes(sha256).as_slice()
    );
    assert_eq!(record.canonical_topology_bytes(), body);
}

fn assert_counts(history: &PublicationHistory, records: usize, bytes: usize) {
    assert_eq!(history.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(history.limits(), HISTORY_LIMITS);
    assert_eq!(history.keys_len(), 1);
    assert_eq!(history.records().len(), records);
    assert_eq!(history.canonical_bytes_len(), bytes);
}

fn assert_latest(history: &PublicationHistory, index: usize) {
    assert_eq!(
        history.latest(
            HistoryId::from_bytes([0x22; 16]),
            AccountId::from_bytes([0x33; 16]),
            EntityId::from_bytes([0x44; 16]),
        ),
        Some(&history.records()[index])
    );
}

#[test]
fn immutable_versions_and_tombstone_preserve_canonical_history() {
    let topology_limits = Limits::default();
    let a1_bytes = literal_bytes(include_str!("vectors/system-topology-v1.hex"));
    let a2_bytes = literal_bytes(A2_HEX);
    assert_eq!(a1_bytes.len(), 199);
    assert_eq!(a2_bytes.len(), 199);
    let a1 = decode(&a1_bytes, topology_limits).unwrap();
    let a2 = decode(&a2_bytes, topology_limits).unwrap();
    let originals = [a1.clone(), a2.clone()];
    assert_eq!(a1.revision, 1);
    assert_eq!(a2.revision, 2);
    assert_eq!(a1.campaign_day_millis, 2);
    assert_eq!(a2.campaign_day_millis, 2);
    assert_input(&a1, &originals[0], &a1_bytes, A1_SHA256);
    assert_input(&a2, &originals[1], &a2_bytes, A2_SHA256);

    let mut history =
        PublicationHistory::new(UniverseId::from_bytes([0x11; 16]), HISTORY_LIMITS).unwrap();
    assert_eq!(
        history.append_publication(&a1, topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_counts(&history, 1, 199);
    assert_record(
        &history.records()[0],
        PublicationKind::PublishedVersion,
        1,
        1,
        A1_SHA256,
        Some(&a1_bytes),
    );
    assert_latest(&history, 0);
    let first_history = history.clone();
    let first_record = history.records()[0].clone();
    let first_body = history.records()[0]
        .canonical_topology_bytes()
        .unwrap()
        .to_vec();
    let first_hash = history.records()[0].referenced_topology_sha256();

    assert_eq!(
        history.append_publication(&a2, topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_counts(&history, 2, 398);
    assert_eq!(history.records()[0], first_record);
    assert_record(
        &history.records()[1],
        PublicationKind::PublishedVersion,
        2,
        2,
        A2_SHA256,
        Some(&a2_bytes),
    );
    assert_latest(&history, 1);
    let second_history = history.clone();
    let second_records = history.records().to_vec();
    assert_eq!(
        history.canonical_bytes_len(),
        history.limits().canonical_bytes
    );

    assert_eq!(
        history.append_tombstone(
            HistoryId::from_bytes([0x22; 16]),
            AccountId::from_bytes([0x33; 16]),
            EntityId::from_bytes([0x44; 16]),
            3,
        ),
        Ok(PublicationOutcome::TombstoneAppended)
    );
    assert_counts(&history, 3, 398);
    assert_record(
        &history.records()[0],
        PublicationKind::PublishedVersion,
        1,
        1,
        A1_SHA256,
        Some(&a1_bytes),
    );
    assert_record(
        &history.records()[1],
        PublicationKind::PublishedVersion,
        2,
        2,
        A2_SHA256,
        Some(&a2_bytes),
    );
    assert_record(
        &history.records()[2],
        PublicationKind::Tombstone,
        3,
        2,
        A2_SHA256,
        None,
    );
    assert_latest(&history, 2);
    assert_eq!(first_record, history.records()[0]);
    assert_counts(&first_history, 1, 199);
    assert_eq!(first_history.records(), std::slice::from_ref(&first_record));
    assert_eq!(first_body, a1_bytes);
    assert_eq!(first_hash.as_slice(), literal_bytes(A1_SHA256).as_slice());
    assert_counts(&second_history, 2, 398);
    assert_eq!(second_history.records(), second_records.as_slice());
    assert_eq!(&history.records()[..2], second_records.as_slice());
    assert_input(&a1, &originals[0], &a1_bytes, A1_SHA256);
    assert_input(&a2, &originals[1], &a2_bytes, A2_SHA256);
}
