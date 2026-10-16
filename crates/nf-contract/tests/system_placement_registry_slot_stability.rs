use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementError, PlacementRegistry, ReservationOutcome},
    system_topology::{Limits, Topology, decode, digest, encode, validate},
};

// Independent fixed NF-SPT-1 inputs; the registry does not create these expected bytes.
const A1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);

const A2_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440200000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);

const HIGH_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440000000000",
    "000080ffffffffffffffff0101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);

const A1_SHA: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const A2_SHA: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
const HIGH_SHA: &str = "840c1706efbc0e083b7743c01e9964f65b7698767e5bb5f983e7a2762b77fc53";

fn literal(hex: &str) -> Vec<u8> {
    assert!(matches!(hex.len(), 398 | 64));
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_assignment(registry: &PlacementRegistry) {
    assert_eq!(registry.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(registry.capacity(), 2);
    assert_eq!(registry.entries().len(), 1);
    let entry = &registry.entries()[0];
    assert_eq!(entry.history(), HistoryId::from_bytes([0x22; 16]));
    assert_eq!(entry.owner(), AccountId::from_bytes([0x33; 16]));
    assert_eq!(entry.system(), EntityId::from_bytes([0x44; 16]));
    assert_eq!(entry.slot().value(), 7);
    assert_eq!(entry.revision(), 2);
    assert_eq!(
        entry.topology_sha256().as_slice(),
        literal(A2_SHA).as_slice()
    );
}

fn assert_input(topology: &Topology, original: &[u8], expected_sha: &str) {
    let limits = Limits::default();
    assert_eq!(original.len(), 199);
    assert_eq!(validate(topology, limits), Ok(()));
    assert_eq!(encode(topology, limits).unwrap().as_slice(), original);
    assert_eq!(
        digest(topology, limits).unwrap().as_slice(),
        literal(expected_sha).as_slice()
    );
}

#[test]
fn existing_key_refuses_slot_change_before_revision_classification() {
    let limits = Limits::default();
    let a1_bytes = literal(A1_HEX);
    let a2_bytes = literal(A2_HEX);
    let high_bytes = literal(HIGH_HEX);
    let a1 = decode(&a1_bytes, limits).unwrap();
    let a2 = decode(&a2_bytes, limits).unwrap();
    let high = decode(&high_bytes, limits).unwrap();
    let a1_before = a1.clone();
    let a2_before = a2.clone();
    let high_before = high.clone();
    assert_input(&a1, &a1_bytes, A1_SHA);
    assert_input(&a2, &a2_bytes, A2_SHA);
    assert_input(&high, &high_bytes, HIGH_SHA);
    assert_eq!(a1.revision, 1);
    assert_eq!(a2.revision, 2);
    assert_eq!(high.revision, 1_u64 << 63);
    assert_eq!(high.campaign_day_millis, u64::MAX);

    let mut registry = PlacementRegistry::new(UniverseId::from_bytes([0x11; 16]), 2).unwrap();
    assert_eq!(
        registry.reserve(&a2, ConceptualSlot::new(7), limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_assignment(&registry);
    let before_refusals = registry.clone();
    // Slot8 is free: only the existing key's immutable slot binding can refuse it.
    assert!(registry.entries().iter().all(|entry| entry.slot().value() != 8));

    assert_eq!(
        registry.reserve(&a1, ConceptualSlot::new(8), limits),
        Err(PlacementError::SlotMismatch)
    );
    assert_eq!(registry, before_refusals);
    assert_assignment(&registry);
    assert_eq!(a1, a1_before);
    assert_eq!(a2, a2_before);
    assert_eq!(high, high_before);

    assert_eq!(
        registry.reserve(&a2, ConceptualSlot::new(8), limits),
        Err(PlacementError::SlotMismatch)
    );
    assert_eq!(registry, before_refusals);
    assert_assignment(&registry);
    assert_eq!(a1, a1_before);
    assert_eq!(a2, a2_before);
    assert_eq!(high, high_before);

    assert_eq!(
        registry.reserve(&high, ConceptualSlot::new(8), limits),
        Err(PlacementError::SlotMismatch)
    );
    assert_eq!(registry, before_refusals);
    assert_assignment(&registry);
    assert_eq!(a1, a1_before);
    assert_eq!(a2, a2_before);
    assert_eq!(high, high_before);
    assert_input(&a1, &a1_bytes, A1_SHA);
    assert_input(&a2, &a2_bytes, A2_SHA);
    assert_input(&high, &high_bytes, HIGH_SHA);
    assert_eq!(a1_bytes, literal(A1_HEX));
    assert_eq!(a2_bytes, literal(A2_HEX));
    assert_eq!(high_bytes, literal(HIGH_HEX));
}
