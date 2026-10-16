use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementError, PlacementRegistry, ReservationOutcome},
    system_topology::{Limits, Topology, decode, digest, encode, validate},
};

// Independent fixed NF-SPT-1 inputs, not expected bytes from the registry.
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

const MAX_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "2222223333333333333333333333333333333344444444444444444444444444444444ffffffffff",
    "ffffffffffffffffffffff0101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);

const A2_SHA: &str = "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";
const HIGH_SHA: &str = "840c1706efbc0e083b7743c01e9964f65b7698767e5bb5f983e7a2762b77fc53";
const MAX_SHA: &str = "af7535b29a044a03adf92107775dd55b2c4e14f03fac34ecca4e216d53151345";

fn literal(hex: &str) -> Vec<u8> {
    assert!(matches!(hex.len(), 398 | 64));
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_assignment(registry: &PlacementRegistry, revision: u64, expected_sha: &str) {
    assert_eq!(registry.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(registry.capacity(), 2);
    assert_eq!(registry.entries().len(), 1);
    let entry = &registry.entries()[0];
    assert_eq!(entry.history(), HistoryId::from_bytes([0x22; 16]));
    assert_eq!(entry.owner(), AccountId::from_bytes([0x33; 16]));
    assert_eq!(entry.system(), EntityId::from_bytes([0x44; 16]));
    assert_eq!(entry.slot().value(), 7);
    assert_eq!(entry.revision(), revision);
    assert_eq!(entry.topology_sha256().as_slice(), literal(expected_sha).as_slice());
}

fn assert_input(topology: &Topology, original: &[u8], expected_sha: &str) {
    let limits = Limits::default();
    assert_eq!(original.len(), 199);
    assert_eq!(validate(topology, limits), Ok(()));
    assert_eq!(encode(topology, limits).unwrap().as_slice(), original);
    assert_eq!(digest(topology, limits).unwrap().as_slice(), literal(expected_sha).as_slice());
}

#[test]
fn unsigned_revisions_advance_through_u64_max_and_reject_older_values() {
    let limits = Limits::default();
    let a2_bytes = literal(A2_HEX);
    let high_bytes = literal(HIGH_HEX);
    let max_bytes = literal(MAX_HEX);
    let a2 = decode(&a2_bytes, limits).unwrap();
    let high = decode(&high_bytes, limits).unwrap();
    let maximum = decode(&max_bytes, limits).unwrap();
    assert_input(&a2, &a2_bytes, A2_SHA);
    assert_input(&high, &high_bytes, HIGH_SHA);
    assert_input(&maximum, &max_bytes, MAX_SHA);
    assert_eq!(a2.revision, 2);
    assert_eq!(high.revision, 1_u64 << 63);
    assert_eq!(high.campaign_day_millis, u64::MAX);
    assert_eq!(maximum.revision, u64::MAX);
    assert_eq!(maximum.campaign_day_millis, u64::MAX);
    let slot = ConceptualSlot::new(7);
    let mut registry = PlacementRegistry::new(UniverseId::from_bytes([0x11; 16]), 2).unwrap();
    assert_eq!(
        registry.reserve(&a2, slot, limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_assignment(&registry, 2, A2_SHA);
    assert_eq!(
        registry.reserve(&high, slot, limits),
        Ok(ReservationOutcome::Updated)
    );
    assert_assignment(&registry, 1_u64 << 63, HIGH_SHA);
    assert_eq!(
        registry.reserve(&maximum, slot, limits),
        Ok(ReservationOutcome::Updated)
    );
    assert_assignment(&registry, u64::MAX, MAX_SHA);
    let before_repeat = registry.clone();
    assert_eq!(
        registry.reserve(&maximum, slot, limits),
        Ok(ReservationOutcome::Unchanged)
    );
    assert_eq!(registry, before_repeat);
    assert_assignment(&registry, u64::MAX, MAX_SHA);
    let before_stale = registry.clone();
    assert_eq!(
        registry.reserve(&high, slot, limits),
        Err(PlacementError::StaleRevision)
    );
    assert_eq!(registry, before_stale);
    assert_assignment(&registry, u64::MAX, MAX_SHA);
    assert_eq!(
        registry.reserve(&a2, slot, limits),
        Err(PlacementError::StaleRevision)
    );
    assert_eq!(registry, before_stale);
    assert_assignment(&registry, u64::MAX, MAX_SHA);
    assert_input(&a2, &a2_bytes, A2_SHA);
    assert_input(&high, &high_bytes, HIGH_SHA);
    assert_input(&maximum, &max_bytes, MAX_SHA);
    assert_eq!(a2_bytes, literal(A2_HEX));
    assert_eq!(high_bytes, literal(HIGH_HEX));
    assert_eq!(max_bytes, literal(MAX_HEX));
}
