mod support;
use nf_contract::identity::*;
use nf_world::*;
fn golden() -> Vec<u8> {
    let s = include_str!("../fixtures/genesis-component.hex").trim();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn typed_component5_matches_independent_exact_golden_and_roundtrips() {
    let s = support::state();
    let bytes = encode_component(&s, WorldTick(0)).expect("registered private component5");
    assert_eq!(bytes, golden());
    assert_eq!(bytes.len(), 1018);
    assert_eq!(decode_component(&bytes, WorldTick(0)).unwrap(), s);
}
