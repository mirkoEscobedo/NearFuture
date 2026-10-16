use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementError, PlacementRegistry, ReservationOutcome},
    system_topology::{Limits, decode, digest, encode, validate},
};

const ORIGINAL_SHA256: &str =
    "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const REVISION_TWO_SHA256: &str =
    "81e2a157a78c5e53f9334e9f3adb690767d0fb8e5aa651c11da30bbf7399878d";

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

fn assert_assignment(registry: &PlacementRegistry, revision: u64, expected_sha256: &str) {
    assert_eq!(registry.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(registry.capacity(), 2);
    assert_eq!(registry.entries().len(), 1);
    let entry = &registry.entries()[0];
    assert_eq!(entry.history(), HistoryId::from_bytes([0x22; 16]));
    assert_eq!(entry.owner(), AccountId::from_bytes([0x33; 16]));
    assert_eq!(entry.system(), EntityId::from_bytes([0x44; 16]));
    assert_eq!(entry.slot().value(), 7);
    assert_eq!(entry.revision(), revision);
    assert_eq!(
        entry.topology_sha256().as_slice(),
        literal_bytes(expected_sha256).as_slice()
    );
}

#[test]
fn slot_collision_refuses_without_changing_existing_assignment() {
    let limits = Limits::default();
    let original_bytes = literal_bytes(include_str!("vectors/system-topology-v1.hex"));
    assert_eq!(original_bytes.len(), 199);
    let original = decode(&original_bytes, limits).unwrap();
    assert_eq!(validate(&original, limits), Ok(()));
    assert_eq!(
        digest(&original, limits).unwrap().as_slice(),
        literal_bytes(ORIGINAL_SHA256).as_slice()
    );

    let slot = ConceptualSlot::new(7);
    let mut registry = PlacementRegistry::new(UniverseId::from_bytes([0x11; 16]), 2).unwrap();
    assert_eq!(
        registry.reserve(&original, slot, limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_assignment(&registry, 1, ORIGINAL_SHA256);

    let mut updated = original.clone();
    updated.revision = 2;
    assert_eq!(validate(&updated, limits), Ok(()));
    assert_eq!(
        digest(&updated, limits).unwrap().as_slice(),
        literal_bytes(REVISION_TWO_SHA256).as_slice()
    );
    assert_eq!(
        registry.reserve(&updated, slot, limits),
        Ok(ReservationOutcome::Updated)
    );
    assert_assignment(&registry, 2, REVISION_TWO_SHA256);

    let mut other = original.clone();
    other.system = EntityId::from_bytes([0x55; 16]);
    assert_eq!(validate(&other, limits), Ok(()));
    let before_collision = registry.clone();
    assert_eq!(
        registry.reserve(&other, slot, limits),
        Err(PlacementError::SlotOccupied)
    );
    assert_eq!(registry, before_collision);
    assert_assignment(&registry, 2, REVISION_TWO_SHA256);
    assert_eq!(encode(&original, limits).unwrap(), original_bytes);
    assert_eq!(
        digest(&original, limits).unwrap().as_slice(),
        literal_bytes(ORIGINAL_SHA256).as_slice()
    );
}
