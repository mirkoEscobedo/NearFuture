use nf_wire::{WireError, decode_control};
#[test]
fn rejects_duplicate_singular_before_generated_last_value_wins() {
    assert_eq!(
        decode_control(&[0x08, 1, 0x08, 1]),
        Err(WireError::Duplicate)
    );
}
