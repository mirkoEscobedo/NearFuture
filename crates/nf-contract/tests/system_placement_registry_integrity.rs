use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementError, PlacementRegistry, ReservationOutcome},
    system_topology::{Error as TopologyError, Limits, Topology, decode, digest, encode, validate},
};

// Fixed independent NF-SPT-1 bytes and raw SHA256; no registry-generated expectations.
const A1_HEX: &str = concat!(
    "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222",
    "222222223333333333333333333333333333333344444444444444444444444444444444010000",
    "000000000002000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const A1_SHA: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";

const A2_HEX: &str = concat!(
    "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222",
    "222222223333333333333333333333333333333344444444444444444444444444444444020000",
    "000000000002000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const A2_SHA: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";

const CONFLICT_HEX: &str = concat!(
    "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222",
    "222222223333333333333333333333333333333344444444444444444444444444444444020000",
    "000000000003000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const CONFLICT_SHA: &str = "52ed6079f4b31e1aa71f17a3740bfd085f56bc6bd8f4df1bb4ec31ae136fa5be";

const FOREIGN_HEX: &str = concat!(
    "4e462d5350542d3100010066666666666666666666666666666666222222222222222222222222",
    "222222223333333333333333333333333333333344444444444444444444444444444444010000",
    "000000000002000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const FOREIGN_SHA: &str = "d5ef8f6cf43605f678de37a6a247129885616d6748f104d15d3330b9926664ae";

const SYSTEM_HIGH_HEX: &str = concat!(
    "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222",
    "222222223333333333333333333333333333333380808080808080808080808080808080010000",
    "000000000002000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const SYSTEM_HIGH_SHA: &str = "c2dacf84b51533e15caedd89d249aa45907d85771fd4e226bdb481fb6172e5d0";

const OWNER_HIGH_HEX: &str = concat!(
    "4e462d5350542d3100010011111111111111111111111111111111222222222222222222222222",
    "222222228080808080808080808080808080808044444444444444444444444444444444010000",
    "000000000002000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const OWNER_HIGH_SHA: &str = "7c84b1dcc28bcf9e396610d622dc76f18e94fde8f66fce8102d76cc4bcdc98cc";

const HISTORY_HIGH_HEX: &str = concat!(
    "4e462d5350542d3100010011111111111111111111111111111111808080808080808080808080",
    "808080803333333333333333333333333333333344444444444444444444444444444444010000",
    "000000000002000000000000000101007777777777777777777777777777777777777777777777",
    "7777777777777777770102000101010101010101010101010101010101000001030053756e0202",
    "020202020202020202020202020202010101010101010101010101010101010101000001050057",
    "6f726c64",
);
const HISTORY_HIGH_SHA: &str = "6af9fefed8d8d7a320c62fef00b3126a1285259245dbcea2dcb06fc4c7e01649";

const FIXTURES: [(&str, &str); 7] = [
    (A1_HEX, A1_SHA),
    (A2_HEX, A2_SHA),
    (CONFLICT_HEX, CONFLICT_SHA),
    (FOREIGN_HEX, FOREIGN_SHA),
    (SYSTEM_HIGH_HEX, SYSTEM_HIGH_SHA),
    (OWNER_HIGH_HEX, OWNER_HIGH_SHA),
    (HISTORY_HIGH_HEX, HISTORY_HIGH_SHA),
];

fn literal(hex: &str) -> Vec<u8> {
    assert!(matches!(hex.len(), 398 | 64));
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_fixed(inputs: &[Topology]) {
    assert!(inputs.len() >= FIXTURES.len());
    for (input, (hex, sha)) in inputs.iter().zip(FIXTURES) {
        let bytes = literal(hex);
        assert_eq!(bytes.len(), 199);
        assert_eq!(validate(input, Limits::default()), Ok(()));
        assert_eq!(encode(input, Limits::default()).unwrap(), bytes);
        assert_eq!(
            digest(input, Limits::default()).unwrap().as_slice(),
            literal(sha)
        );
    }
}

fn fixtures() -> Vec<Topology> {
    let inputs: Vec<_> = FIXTURES
        .iter()
        .map(|(hex, _)| decode(&literal(hex), Limits::default()).unwrap())
        .collect();
    assert_fixed(&inputs);
    inputs
}

fn refuse(
    registry: &mut PlacementRegistry,
    inputs: &[Topology],
    index: usize,
    slot: u16,
    limits: Limits,
    error: PlacementError,
) {
    let before = registry.clone();
    let inputs_before = inputs.to_vec();
    let limits_before = (
        limits.entities,
        limits.dependencies,
        limits.label_bytes,
        limits.document_bytes,
    );
    assert_eq!(
        registry.reserve(&inputs[index], ConceptualSlot::new(slot), limits),
        Err(error)
    );
    assert_eq!(*registry, before);
    assert_eq!(inputs, inputs_before);
    assert_eq!(
        (
            limits.entities,
            limits.dependencies,
            limits.label_bytes,
            limits.document_bytes,
        ),
        limits_before
    );
    // Only the first seven are valid canonical fixtures. Invalid values use full Eq above.
    assert_fixed(inputs);
}

#[test]
fn conflict_and_validation_refusals_preserve_registry_and_all_inputs() {
    let mut inputs = fixtures();
    let mut invalid_foreign = inputs[3].clone();
    invalid_foreign.revision = 0;
    let mut invalid_same_key = inputs[1].clone();
    invalid_same_key.revision = 0;
    inputs.extend([invalid_foreign, invalid_same_key]);
    let limits = Limits::default();
    let mut registry = PlacementRegistry::new(UniverseId::from_bytes([0x11; 16]), 1).unwrap();
    assert_eq!(
        registry.reserve(&inputs[1], ConceptualSlot::new(7), limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_eq!(registry.entries().len(), 1);
    assert_eq!(registry.entries()[0].revision(), 2);
    assert_eq!(
        registry.entries()[0].topology_sha256().as_slice(),
        literal(A2_SHA)
    );
    assert_eq!(inputs[2].campaign_day_millis, 3);
    assert_eq!(inputs[2].revision, 2);
    refuse(
        &mut registry, &inputs, 2, 7, limits, PlacementError::RevisionConflict,
    );
    assert_eq!(inputs[0].revision, 1);
    refuse(&mut registry, &inputs, 0, 7, limits, PlacementError::StaleRevision);

    // Actual reserve validates revision0 before a real foreign-universe refusal.
    assert_ne!(inputs[7].universe, registry.universe());
    assert_eq!(inputs[7].revision, 0);
    refuse(
        &mut registry, &inputs, 7, 7, limits,
        PlacementError::Topology(TopologyError::Invalid),
    );
    // Actual above-default limits precede the same valid foreign-universe refusal.
    let above_default = Limits {
        entities: 129,
        ..limits
    };
    assert_eq!(validate(&inputs[3], limits), Ok(()));
    assert_ne!(inputs[3].universe, registry.universe());
    refuse(
        &mut registry, &inputs, 3, 7, above_default,
        PlacementError::Topology(TopologyError::Limit),
    );
    // Matching key with revision0 requests slot8, different from the stored slot7.
    assert_eq!(inputs[8].history, inputs[1].history);
    assert_eq!(inputs[8].owner, inputs[1].owner);
    assert_eq!(inputs[8].system, inputs[1].system);
    assert_eq!(registry.entries()[0].slot().value(), 7);
    refuse(
        &mut registry, &inputs, 8, 8, limits,
        PlacementError::Topology(TopologyError::Invalid),
    );
    refuse(&mut registry, &inputs, 3, 7, limits, PlacementError::UniverseMismatch);
}

fn assert_key_rows(registry: &PlacementRegistry, revisions: [u64; 4]) {
    // Independent fixed order, not sorted from registry or input iteration order.
    let expected = [
        (0x22, 0x33, 0x44, 9, A1_SHA),
        (0x22, 0x33, 0x80, 3, SYSTEM_HIGH_SHA),
        (0x22, 0x80, 0x44, 12, OWNER_HIGH_SHA),
        (0x80, 0x33, 0x44, 5, HISTORY_HIGH_SHA),
    ];
    assert_eq!(registry.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(registry.capacity(), 4);
    assert_eq!(registry.entries().len(), 4);
    for (index, (history, owner, system, slot, sha)) in expected.into_iter().enumerate() {
        let entry = &registry.entries()[index];
        assert_eq!(entry.history(), HistoryId::from_bytes([history; 16]));
        assert_eq!(entry.owner(), AccountId::from_bytes([owner; 16]));
        assert_eq!(entry.system(), EntityId::from_bytes([system; 16]));
        assert_eq!(entry.slot().value(), slot);
        assert_eq!(entry.revision(), revisions[index]);
        let expected_sha = if index == 0 && revisions[0] == 2 {
            A2_SHA
        } else {
            sha
        };
        assert_eq!(entry.topology_sha256().as_slice(), literal(expected_sha));
    }
}

#[test]
fn canonical_full_keys_and_owned_snapshots_survive_updates_and_copy_mutation() {
    let mut inputs = fixtures();
    let mut system_source = [0x80; 16];
    let mut owner_source = [0x80; 16];
    let mut history_source = [0x80; 16];
    inputs[4].system = EntityId::from_bytes(system_source);
    inputs[5].owner = AccountId::from_bytes(owner_source);
    inputs[6].history = HistoryId::from_bytes(history_source);
    let inputs_before = inputs.clone();
    let limits = Limits::default();
    let universe = UniverseId::from_bytes([0x11; 16]);
    let mut first = PlacementRegistry::new(universe, 4).unwrap();
    let mut second = PlacementRegistry::new(universe, 4).unwrap();
    for (index, slot) in [(6, 5), (4, 3), (5, 12), (0, 9)] {
        assert_eq!(
            first.reserve(&inputs[index], ConceptualSlot::new(slot), limits),
            Ok(ReservationOutcome::Inserted)
        );
    }
    for (index, slot) in [(5, 12), (0, 9), (6, 5), (4, 3)] {
        assert_eq!(
            second.reserve(&inputs[index], ConceptualSlot::new(slot), limits),
            Ok(ReservationOutcome::Inserted)
        );
    }
    assert_key_rows(&first, [1; 4]);
    assert_key_rows(&second, [1; 4]);
    assert_eq!(first, second);
    assert_eq!(inputs, inputs_before);
    let old_rows = first.entries().to_vec();
    let old_entry = first.entries()[0].clone();
    let old_hash = old_entry.topology_sha256();
    let before = first.clone();
    let mut returned_hash = first.entries()[0].topology_sha256();
    returned_hash.fill(0);
    system_source.fill(0x01);
    owner_source.fill(0x02);
    history_source.fill(0x03);
    assert_eq!(returned_hash, [0; 32]);
    assert_eq!(system_source, [0x01; 16]);
    assert_eq!(owner_source, [0x02; 16]);
    assert_eq!(history_source, [0x03; 16]);
    assert_eq!(inputs, inputs_before);
    assert_eq!(first, before);
    assert_key_rows(&first, [1; 4]);

    assert_eq!(
        first.reserve(&inputs[1], ConceptualSlot::new(9), limits),
        Ok(ReservationOutcome::Updated)
    );
    assert_key_rows(&first, [2, 1, 1, 1]);
    assert_key_rows(&second, [1; 4]);
    assert_eq!(old_rows.as_slice(), second.entries());
    assert_eq!(old_entry, old_rows[0]);
    assert_eq!(old_entry.revision(), 1);
    assert_eq!(old_entry.topology_sha256(), old_hash);
    assert_eq!(old_hash.as_slice(), literal(A1_SHA));
    assert_eq!(inputs, inputs_before);
    assert_fixed(&inputs);
    let after_update = first.clone();
    inputs[0].history = HistoryId::from_bytes([0x66; 16]);
    inputs[4].system = EntityId::from_bytes([0x66; 16]);
    inputs[5].owner = AccountId::from_bytes([0x66; 16]);
    inputs[6].dependencies[0].semantic_key.fill(0x66);
    assert_eq!(first, after_update);
    assert_eq!(old_rows.as_slice(), second.entries());
    assert_key_rows(&first, [2, 1, 1, 1]);
}
