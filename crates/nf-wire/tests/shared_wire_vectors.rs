use nf_wire::{WireError, decode_control};
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn error(s: &str) -> WireError {
    match s {
        "MALFORMED" => WireError::Malformed,
        "DUPLICATE" => WireError::Duplicate,
        "UNKNOWN_FIELD" => WireError::UnknownField,
        "LIMIT" => WireError::Limit,
        "UNSUPPORTED" => WireError::Unsupported,
        "SEMANTIC" => WireError::Semantic,
        _ => panic!("unknown fixture classification"),
    }
}
#[test]
fn shared_raw_wire_corpus_obeys_strict_admission() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../protocol/vectors/wire-v1.json")).unwrap();
    for case in corpus["positive"].as_array().unwrap() {
        assert!(
            decode_control(&unhex(case["wire_hex"].as_str().unwrap())).is_ok(),
            "{}",
            case["name"]
        );
    }
    for case in corpus["malformed"].as_array().unwrap() {
        assert_eq!(
            decode_control(&unhex(case["wire_hex"].as_str().unwrap())),
            Err(error(case["error"].as_str().unwrap())),
            "{}",
            case["name"]
        );
    }
}
