use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_publication::{
        PublicationHistory, PublicationKind, PublicationLimits, PublicationOutcome,
        PublicationProfile,
    },
    system_topology::{Limits, Topology, decode, digest, encode, validate},
};

const A1_SHA256: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const A1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const A2_SHA256: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
const A2_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440200000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const A4_SHA256: &str = "1fa957a0c4e5780f76d86618a6617482c9ce7fff1bad4d534c85085423dc11e1";
const A4_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440400000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);
const HISTORY_LIMITS: PublicationLimits = PublicationLimits {
    keys: 1,
    records: 4,
    canonical_bytes: 597,
};

struct ExpectedRecord<'a> {
    kind: PublicationKind,
    revision: u64,
    referenced_revision: u64,
    sha256: &'static str,
    body: Option<&'a [u8]>,
}

fn literal_bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_inputs(inputs: &[Topology; 3], originals: &[Topology; 3], fixed: &[Vec<u8>; 3]) {
    assert_eq!(inputs, originals);
    for (index, input) in inputs.iter().enumerate() {
        assert_eq!(fixed[index].len(), 199);
        assert_eq!(input.revision, [1, 2, 4][index]);
        assert_eq!(input.campaign_day_millis, 2);
        assert_eq!(validate(input, Limits::default()), Ok(()));
        assert_eq!(encode(input, Limits::default()).unwrap(), fixed[index]);
        assert_eq!(
            digest(input, Limits::default()).unwrap().as_slice(),
            literal_bytes([A1_SHA256, A2_SHA256, A4_SHA256][index]).as_slice()
        );
    }
}

fn assert_state(
    history: &PublicationHistory,
    expected: &[ExpectedRecord<'_>],
    canonical_bytes: usize,
    latest_index: usize,
) {
    assert_eq!(history.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(history.limits(), HISTORY_LIMITS);
    assert_eq!(history.keys_len(), 1);
    assert_eq!(history.canonical_bytes_len(), canonical_bytes);
    assert_eq!(history.records().len(), expected.len());
    for (record, expected) in history.records().iter().zip(expected) {
        assert_eq!(record.universe(), UniverseId::from_bytes([0x11; 16]));
        assert_eq!(record.history(), HistoryId::from_bytes([0x22; 16]));
        assert_eq!(record.owner(), AccountId::from_bytes([0x33; 16]));
        assert_eq!(record.system(), EntityId::from_bytes([0x44; 16]));
        assert_eq!(record.profile(), PublicationProfile::SelectedTopologyV1);
        assert_eq!(record.kind(), expected.kind);
        assert_eq!(record.revision(), expected.revision);
        assert_eq!(
            record.referenced_publication_revision(),
            expected.referenced_revision
        );
        assert_eq!(
            record.referenced_topology_sha256().as_slice(),
            literal_bytes(expected.sha256).as_slice()
        );
        assert_eq!(record.canonical_topology_bytes(), expected.body);
    }
    assert_eq!(
        history.latest(
            HistoryId::from_bytes([0x22; 16]),
            AccountId::from_bytes([0x33; 16]),
            EntityId::from_bytes([0x44; 16]),
        ),
        Some(&history.records()[latest_index])
    );
}

#[test]
fn historical_replay_preserves_latest_and_tombstone_reference_across_republish() {
    let topology_limits = Limits::default();
    let fixed = [literal_bytes(A1_HEX), literal_bytes(A2_HEX), literal_bytes(A4_HEX)];
    let inputs = [
        decode(&fixed[0], topology_limits).unwrap(),
        decode(&fixed[1], topology_limits).unwrap(),
        decode(&fixed[2], topology_limits).unwrap(),
    ];
    let originals = inputs.clone();
    assert_inputs(&inputs, &originals, &fixed);
    let expected = [
        ExpectedRecord {
            kind: PublicationKind::PublishedVersion,
            revision: 1,
            referenced_revision: 1,
            sha256: A1_SHA256,
            body: Some(fixed[0].as_slice()),
        },
        ExpectedRecord {
            kind: PublicationKind::PublishedVersion,
            revision: 2,
            referenced_revision: 2,
            sha256: A2_SHA256,
            body: Some(fixed[1].as_slice()),
        },
        ExpectedRecord {
            kind: PublicationKind::Tombstone,
            revision: 3,
            referenced_revision: 2,
            sha256: A2_SHA256,
            body: None,
        },
        ExpectedRecord {
            kind: PublicationKind::PublishedVersion,
            revision: 4,
            referenced_revision: 4,
            sha256: A4_SHA256,
            body: Some(fixed[2].as_slice()),
        },
    ];
    let mut history =
        PublicationHistory::new(UniverseId::from_bytes([0x11; 16]), HISTORY_LIMITS).unwrap();
    assert_eq!(
        history.append_publication(&inputs[0], topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_state(&history, &expected[..1], 199, 0);
    assert_inputs(&inputs, &originals, &fixed);
    assert_eq!(
        history.append_publication(&inputs[1], topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_state(&history, &expected[..2], 398, 1);
    assert_inputs(&inputs, &originals, &fixed);
    assert_eq!(
        history.append_tombstone(
            HistoryId::from_bytes([0x22; 16]),
            AccountId::from_bytes([0x33; 16]),
            EntityId::from_bytes([0x44; 16]),
            3,
        ),
        Ok(PublicationOutcome::TombstoneAppended)
    );
    assert_state(&history, &expected[..3], 398, 2);
    assert_inputs(&inputs, &originals, &fixed);

    {
        let before = history.clone();
        let input_images = inputs.clone();
        assert_eq!(
            history.append_publication(&inputs[0], topology_limits),
            Ok(PublicationOutcome::Unchanged)
        );
        assert_eq!(history, before);
        assert_eq!(inputs, input_images);
        assert_state(&history, &expected[..3], 398, 2);
        assert_inputs(&inputs, &originals, &fixed);
    }
    {
        let before = history.clone();
        let input_images = inputs.clone();
        assert_eq!(
            history.append_publication(&inputs[1], topology_limits),
            Ok(PublicationOutcome::Unchanged)
        );
        assert_eq!(history, before);
        assert_eq!(inputs, input_images);
        assert_state(&history, &expected[..3], 398, 2);
        assert_inputs(&inputs, &originals, &fixed);
    }
    {
        let before = history.clone();
        let input_images = inputs.clone();
        assert_eq!(
            history.append_tombstone(
                HistoryId::from_bytes([0x22; 16]),
                AccountId::from_bytes([0x33; 16]),
                EntityId::from_bytes([0x44; 16]),
                3,
            ),
            Ok(PublicationOutcome::Unchanged)
        );
        assert_eq!(history, before);
        assert_eq!(inputs, input_images);
        assert_state(&history, &expected[..3], 398, 2);
        assert_inputs(&inputs, &originals, &fixed);
    }
    let tombstone_snapshot = history.records()[2].clone();
    assert_eq!(
        history.append_publication(&inputs[2], topology_limits),
        Ok(PublicationOutcome::PublicationAppended)
    );
    assert_state(&history, &expected, 597, 3);
    assert_inputs(&inputs, &originals, &fixed);
    assert_eq!(history.records()[2], tombstone_snapshot);
    assert_eq!(history.records().len(), history.limits().records);
    assert_eq!(history.canonical_bytes_len(), history.limits().canonical_bytes);
    {
        let before = history.clone();
        let input_images = inputs.clone();
        assert_eq!(
            history.append_tombstone(
                HistoryId::from_bytes([0x22; 16]),
                AccountId::from_bytes([0x33; 16]),
                EntityId::from_bytes([0x44; 16]),
                3,
            ),
            Ok(PublicationOutcome::Unchanged)
        );
        assert_eq!(history, before);
        assert_eq!(inputs, input_images);
        assert_state(&history, &expected, 597, 3);
        assert_inputs(&inputs, &originals, &fixed);
        assert_eq!(history.records()[2], tombstone_snapshot);
    }
    {
        let before = history.clone();
        let input_images = inputs.clone();
        assert_eq!(
            history.append_publication(&inputs[0], topology_limits),
            Ok(PublicationOutcome::Unchanged)
        );
        assert_eq!(history, before);
        assert_eq!(inputs, input_images);
        assert_state(&history, &expected, 597, 3);
        assert_inputs(&inputs, &originals, &fixed);
        assert_eq!(history.records()[2], tombstone_snapshot);
    }
}
