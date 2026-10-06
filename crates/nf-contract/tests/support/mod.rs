pub fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../protocol/vectors/nf-canon-1.json")).unwrap()
}
pub fn bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
