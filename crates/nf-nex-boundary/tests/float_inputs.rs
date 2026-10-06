use nf_nex_boundary::{BoundaryError, FloatBits};

#[test]
fn nonfinite_java_inputs_are_unavailable_without_reinterpretation() {
    for bits in [0x7f80_0000, 0xff80_0000, 0x7fc0_0001] {
        assert_eq!(FloatBits::new(bits), Err(BoundaryError::NonFinite));
    }
    for bits in [0x8000_0000, 0x0000_0001, 0x3f80_0000] {
        assert_eq!(FloatBits::new(bits).unwrap().bits(), bits);
    }
}
