use nf_wire::{WireError, decode_control};
#[test]
fn empty_envelope_is_not_an_admitted_control_message() {
    assert_eq!(decode_control(&[]), Err(WireError::Semantic));
}
