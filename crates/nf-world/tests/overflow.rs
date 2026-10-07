mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn exhausted_aggregate_revision_cannot_produce_a_resource_reservation() {
    let s = support::state();
    let mut bytes = encode_component(&s, WorldTick(0)).unwrap();
    bytes[161..169].copy_from_slice(&u64::MAX.to_le_bytes());
    let max = decode_component(&bytes, WorldTick(0)).unwrap();
    let c = support::colony(&max, 1, 7);
    assert_eq!(
        evaluate(&max, WorldTick(1), &c).err(),
        Some(WorldError::Overflow)
    );
}
