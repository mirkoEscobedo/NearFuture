use nf_nex_boundary::{FloatBits, Modifier, ModifierKind};
use nf_nex_shadow::{Unavailable, make_peace_weariness_modifier};

fn write(weariness: u32, minimum: u32) -> Result<Modifier, Unavailable> {
    make_peace_weariness_modifier(
        FloatBits::new(weariness).unwrap(),
        FloatBits::new(minimum).unwrap(),
    )
}
fn literal(value: u32) -> Result<Modifier, Unavailable> {
    Ok(Modifier {
        id: "weariness".into(),
        kind: ModifierKind::Multiplier,
        value: FloatBits::new(value).unwrap(),
    })
}

#[test]
fn above_cap_writes_literal_five() {
    // Independent literal: source min(12f / 2f, 5f), not an expected-answer helper calculation.
    assert_eq!(write(0x41400000, 0x40000000), literal(0x40a00000));
}
#[test]
fn below_cap_keeps_fractional_ratio() {
    assert_eq!(write(0x40600000, 0x40000000), literal(0x3fe00000));
}
#[test]
fn exactly_at_cap_keeps_five() {
    assert_eq!(write(0x41200000, 0x40000000), literal(0x40a00000));
}
#[test]
fn negative_ratio_is_not_clamped_to_zero() {
    assert_eq!(write(0xc0800000, 0x40000000), literal(0xc0000000));
}
#[test]
fn positive_and_negative_zero_keep_raw_sign() {
    assert_eq!(write(0x00000000, 0x40000000), literal(0x00000000));
    assert_eq!(write(0x80000000, 0x40000000), literal(0x80000000));
}
#[test]
fn overflowing_positive_ratio_is_capped_before_output_validation() {
    // Both inputs are finite; source float division overflows, then Math.min returns five.
    assert_eq!(write(0x7f7fffff, 0x00000001), literal(0x40a00000));
}
#[test]
fn diagnostic_boundary_refuses_nonpositive_minimum() {
    // Explicit copied-fact domain limit, not a claimed Nex source refusal policy.
    for minimum in [0x00000000, 0x80000000, 0xc0000000] {
        assert_eq!(write(0x41400000, minimum), Err(Unavailable::Unsupported));
    }
}
#[test]
fn diagnostic_boundary_refuses_unrepresentable_negative_output() {
    // Source would write negative infinity; this finite Modifier trace cannot represent it.
    assert_eq!(write(0xff7fffff, 0x00000001), Err(Unavailable::NonFinite));
}
