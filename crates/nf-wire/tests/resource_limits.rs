use nf_wire::{Limits, WireError, decode_control, decode_control_with_limits, decode_frame};
fn valid() -> Vec<u8> {
    vec![
        8, 1, 18, 9, 9, 1, 0, 0, 0, 0, 0, 0, 0, 26, 0, 146, 1, 7, 8, 1, 18, 3, 98, 97, 100,
    ]
}
#[test]
fn lower_resource_budgets_fail_before_generated_decoding() {
    let base = Limits::default();
    for limits in [
        Limits {
            frame_bytes: 24,
            ..base
        },
        Limits {
            decoded_bytes: 64,
            ..base
        },
        Limits { depth: 1, ..base },
        Limits {
            total_entries: 1,
            ..base
        },
    ] {
        assert_eq!(
            decode_control_with_limits(&valid(), limits),
            Err(WireError::Limit)
        );
    }
    assert_eq!(
        decode_control_with_limits(&valid(), Limits { depth: 33, ..base }),
        Err(WireError::Limit)
    );
    assert_eq!(decode_control(&vec![0; 1_048_577]), Err(WireError::Limit));
}
#[test]
fn exact_frame_size_and_declared_length_are_checked_first() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../protocol/vectors/wire-v1.json")).unwrap();
    for case in corpus["frame_malformed"].as_array().unwrap() {
        let hex = case["frame_hex"].as_str().unwrap();
        let bytes: Vec<_> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let expected = if case["error"] == "LIMIT" {
            WireError::Limit
        } else {
            WireError::Malformed
        };
        assert_eq!(decode_frame(&bytes), Err(expected), "{}", case["name"]);
    }
    let body = valid();
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend(&body);
    assert!(decode_frame(&frame).is_ok());
    frame.push(0);
    assert_eq!(decode_frame(&frame), Err(WireError::Malformed));
}
#[test]
fn errors_are_public_bounded_static_reasons() {
    for error in [
        WireError::Malformed,
        WireError::Duplicate,
        WireError::UnknownField,
        WireError::Limit,
        WireError::Unsupported,
        WireError::Semantic,
    ] {
        let explanation = error.bounded_error();
        assert!(explanation.reason.len() <= 512);
        assert!(explanation.code > 0);
        assert!(!explanation.retryable);
        assert!(explanation.unsupported_schema_ids.is_empty());
    }
}
