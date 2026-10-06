use nf_contract::identity::*;
use nf_kernel::*;
#[test]
fn empty_snapshot_roundtrips_with_closed_versioned_header() {
    let world = World::new(WorldSpec::empty(
        UniverseId::from_bytes([1; 16]),
        HistoryId::from_bytes([2; 16]),
        [3; 32],
        [4; 32],
    ))
    .unwrap();
    let bytes = encode_snapshot(&world).unwrap();
    assert_eq!(&bytes[..17], b"NF-CANON-1\0\x07\x00\x01\x00\x01\x00");
    assert_eq!(decode_snapshot(&bytes), Ok(world.clone()));
    let mut invalid = bytes.clone();
    invalid[15] = 2;
    assert_eq!(decode_snapshot(&invalid), Err(Rejection::InvalidValue));
}
mod support;
#[test]
fn unordered_input_becomes_one_canonical_world_and_hash() {
    let world = support::fixture();
    let mut spec = world.to_spec();
    spec.factions.reverse();
    spec.providers.reverse();
    spec.registry.reverse();
    spec.ownership.reverse();
    let reordered = World::new(spec).unwrap();
    assert_eq!(reordered, world);
    assert_eq!(state_hash(&reordered), state_hash(&world));
    assert_eq!(
        decode_snapshot(&encode_snapshot(&world).unwrap()),
        Ok(world)
    );
}
