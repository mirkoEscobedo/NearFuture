use nf_contract::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementError, PlacementRegistry, ReservationOutcome},
    system_topology::{Limits, Topology, decode, digest, encode, validate},
};

// Independent fixed NF-SPT-1 literals; no expected bytes are produced by the registry.
const A1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333444444444444444444444444444444440100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);

const B1_HEX: &str = concat!(
    "4e462d5350542d310001001111111111111111111111111111111122222222222222222222222222",
    "22222233333333333333333333333333333333555555555555555555555555555555550100000000",
    "00000002000000000000000101007777777777777777777777777777777777777777777777777777",
    "7777777777770102000101010101010101010101010101010101000001030053756e020202020202",
    "02020202020202020202020101010101010101010101010101010101010000010500576f726c64",
);

const A1_SHA: &str = "f9865f8074a8e754ee54cb9691b0d377e3c47956b0141f36740a9cee97ef4183";
const B1_SHA: &str = "6ea8fd3113fc299431367483e01f808086cdc28062ac9daf9d03a761fdcde821";

fn literal(hex: &str) -> Vec<u8> {
    assert!(matches!(hex.len(), 398 | 64));
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_assignment(registry: &PlacementRegistry, slot: u16) {
    assert_eq!(registry.universe(), UniverseId::from_bytes([0x11; 16]));
    assert_eq!(registry.capacity(), 1);
    assert_eq!(registry.entries().len(), 1);
    let entry = &registry.entries()[0];
    assert_eq!(entry.history(), HistoryId::from_bytes([0x22; 16]));
    assert_eq!(entry.owner(), AccountId::from_bytes([0x33; 16]));
    assert_eq!(entry.system(), EntityId::from_bytes([0x44; 16]));
    assert_eq!(entry.slot().value(), slot);
    assert_eq!(entry.revision(), 1);
    assert_eq!(
        entry.topology_sha256().as_slice(),
        literal(A1_SHA).as_slice()
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
fn slot_extrema_and_full_capacity_refusals_preserve_assignments() {
    let limits = Limits::default();
    let a1_bytes = literal(A1_HEX);
    let b1_bytes = literal(B1_HEX);
    let a1 = decode(&a1_bytes, limits).unwrap();
    let b1 = decode(&b1_bytes, limits).unwrap();
    let a1_before = a1.clone();
    let b1_before = b1.clone();
    assert_input(&a1, &a1_bytes, A1_SHA);
    assert_input(&b1, &b1_bytes, B1_SHA);
    assert_eq!(b1.system, EntityId::from_bytes([0x55; 16]));
    let universe = UniverseId::from_bytes([0x11; 16]);
    let zero_universe = UniverseId::from_bytes([0; 16]);

    let mut at_zero = PlacementRegistry::new(universe, 1).unwrap();
    let mut at_maximum = PlacementRegistry::new(universe, 1).unwrap();
    assert_eq!(
        at_zero.reserve(&a1, ConceptualSlot::new(0), limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_eq!(
        at_maximum.reserve(&a1, ConceptualSlot::new(u16::MAX), limits),
        Ok(ReservationOutcome::Inserted)
    );
    assert_assignment(&at_zero, 0);
    assert_assignment(&at_maximum, u16::MAX);
    let zero_before = at_zero.clone();
    let maximum_before = at_maximum.clone();

    assert_eq!(
        PlacementRegistry::new(universe, 0),
        Err(PlacementError::Bounds)
    );
    assert_eq!(at_zero, zero_before);
    assert_eq!(at_maximum, maximum_before);
    assert_assignment(&at_zero, 0);
    assert_assignment(&at_maximum, u16::MAX);
    assert_eq!(a1, a1_before);
    assert_eq!(b1, b1_before);

    assert_eq!(
        PlacementRegistry::new(universe, 257),
        Err(PlacementError::Bounds)
    );
    assert_eq!(at_zero, zero_before);
    assert_eq!(at_maximum, maximum_before);
    assert_assignment(&at_zero, 0);
    assert_assignment(&at_maximum, u16::MAX);
    assert_eq!(a1, a1_before);
    assert_eq!(b1, b1_before);

    // Selected constructor precedence: invalid namespace wins over invalid capacity.
    assert_eq!(
        PlacementRegistry::new(zero_universe, 0),
        Err(PlacementError::InvalidNamespace)
    );
    assert_eq!(at_zero, zero_before);
    assert_eq!(at_maximum, maximum_before);
    assert_assignment(&at_zero, 0);
    assert_assignment(&at_maximum, u16::MAX);
    assert_eq!(a1, a1_before);
    assert_eq!(b1, b1_before);
    assert_eq!(universe, UniverseId::from_bytes([0x11; 16]));
    assert_eq!(zero_universe, UniverseId::from_bytes([0; 16]));

    // This new key requests an occupied slot in an already full capacity1 registry.
    assert_eq!(
        at_zero.reserve(&b1, ConceptualSlot::new(0), limits),
        Err(PlacementError::SlotOccupied)
    );
    assert_eq!(at_zero, zero_before);
    assert_eq!(at_maximum, maximum_before);
    assert_assignment(&at_zero, 0);
    assert_assignment(&at_maximum, u16::MAX);
    assert_eq!(a1, a1_before);
    assert_eq!(b1, b1_before);

    // Slot65535 is free in at_zero; the separate at_maximum model is not a global owner.
    assert!(
        at_zero
            .entries()
            .iter()
            .all(|entry| entry.slot().value() != u16::MAX)
    );
    assert_eq!(
        at_zero.reserve(&b1, ConceptualSlot::new(u16::MAX), limits),
        Err(PlacementError::Bounds)
    );
    assert_eq!(at_zero, zero_before);
    assert_eq!(at_maximum, maximum_before);
    assert_assignment(&at_zero, 0);
    assert_assignment(&at_maximum, u16::MAX);
    assert_eq!(a1, a1_before);
    assert_eq!(b1, b1_before);
    assert_input(&a1, &a1_bytes, A1_SHA);
    assert_input(&b1, &b1_bytes, B1_SHA);
    assert_eq!(a1_bytes, literal(A1_HEX));
    assert_eq!(b1_bytes, literal(B1_HEX));
}
