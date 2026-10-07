use nf_wire::{WireError, decode_local_auth};
#[test]
fn auth_envelope_is_distinct_closed_and_rejects_missing_headers() {
    assert_eq!(decode_local_auth(&[]), Err(WireError::Semantic));
}
#[test]
fn independent_raw_auth_corpus_has_exact_width_presence_and_closed_registry() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../protocol/vectors/local-auth-v1.json")).unwrap();
    let bytes = |value: &serde_json::Value| {
        let h = value.as_str().unwrap();
        (0..h.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>()
    };
    for case in corpus["wire_valid"].as_array().unwrap() {
        let encoded = bytes(&case["wireHex"]);
        assert!(decode_local_auth(&encoded).is_ok(), "{}", case["name"]);
        assert!(nf_wire::decode_control(&encoded).is_err());
    }
    for case in corpus["wire_invalid"].as_array().unwrap() {
        let expected = match case["error"].as_str().unwrap() {
            "SEMANTIC" => WireError::Semantic,
            "LIMIT" => WireError::Limit,
            "UNSUPPORTED" => WireError::Unsupported,
            "DUPLICATE" => WireError::Duplicate,
            "UNKNOWN_FIELD" => WireError::UnknownField,
            _ => panic!("fixtureerror"),
        };
        assert_eq!(
            decode_local_auth(&bytes(&case["wireHex"])),
            Err(expected),
            "{}",
            case["name"]
        );
    }
    assert_eq!(decode_local_auth(&vec![0; 4097]), Err(WireError::Limit));
}
