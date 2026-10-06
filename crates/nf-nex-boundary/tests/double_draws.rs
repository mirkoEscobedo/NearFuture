mod support;
use nf_nex_boundary::{BoundaryError, DoubleBits, NexWorld, RandomDraw};

#[test]
fn java_double_draws_reject_nonfinite_values() {
    for bits in [
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0001,
    ] {
        assert_eq!(DoubleBits::new(bits), Err(BoundaryError::NonFinite));
    }
}

#[test]
fn adjacent_java_double_draws_survive_the_immutable_view_distinctly() {
    // These adjacent doubles both become the same f32; replay must retain their difference.
    let expected = [0x3fe0_0000_0000_0000, 0x3fe0_0000_0000_0001];
    assert_eq!(
        f64::from_bits(expected[0]) as f32,
        f64::from_bits(expected[1]) as f32
    );
    let mut snapshot = support::snapshot();
    snapshot.timers.draws = expected
        .iter()
        .enumerate()
        .map(|(ordinal, bits)| RandomDraw {
            purpose: "peace-choice".into(),
            ordinal: ordinal as u32,
            value: DoubleBits::new(*bits).unwrap(),
        })
        .collect();
    let world = NexWorld::admit(snapshot).unwrap();
    assert_eq!(world.snapshot().timers.draws[0].value.bits(), expected[0]);
    assert_eq!(world.snapshot().timers.draws[1].value.bits(), expected[1]);
}
