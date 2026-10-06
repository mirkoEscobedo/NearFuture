use nf_wire::{Limits, decode_control, decode_control_with_limits};
#[test]
fn finite_mutation_and_length_fuzz_is_bounded_and_never_panics() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../protocol/vectors/wire-v1.json")).unwrap();
    let mut seed = 0xa831_93c4u64;
    let limits = Limits {
        frame_bytes: 8192,
        field_bytes: 4096,
        decoded_bytes: 16384,
        collection_items: 64,
        total_entries: 256,
        depth: 8,
        ..Limits::default()
    };
    let mut accepted = 0;
    let mut rejected = 0;
    for case in corpus["positive"].as_array().unwrap() {
        let hex = case["wire_hex"].as_str().unwrap();
        let original: Vec<_> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        assert!(decode_control(&original).is_ok());
        for round in 0..2500 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut input = original.clone();
            let index = seed as usize % input.len();
            match round % 4 {
                0 => input[index] ^= (seed >> 16) as u8,
                1 => input.truncate(index),
                2 => input.insert(index, (seed >> 24) as u8),
                _ => input.extend_from_slice(&seed.to_le_bytes()),
            }
            match decode_control_with_limits(&input, limits) {
                Ok(_) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }
    }
    assert_eq!(accepted + rejected, 10000);
    assert!(
        rejected > 0 && accepted > 0,
        "mutation campaign must include both semantic changes and malformed inputs"
    );
    // Very large declared lengths are small actual inputs; never materialize them.
    for field in [0x1a, 0x12, 0xf2] {
        let input = [field, 0xff, 0xff, 0xff, 0xff, 0x0f];
        assert!(decode_control_with_limits(&input, limits).is_err());
    }
}
