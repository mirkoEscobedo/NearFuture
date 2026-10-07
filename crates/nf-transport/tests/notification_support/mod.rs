use sha2::{Digest, Sha256};
pub struct Vector {
    pub category: &'static str,
    pub name: &'static str,
    pub layer: &'static str,
    pub expectation: &'static str,
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
}
pub fn hex(input: &str) -> Vec<u8> {
    assert!(input.len() <= 16384 && input.len().is_multiple_of(2));
    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let nibble = |b: u8| match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                _ => panic!("noncanonical fixture hex"),
            };
            nibble(pair[0]) * 16 + nibble(pair[1])
        })
        .collect()
}
pub fn vectors() -> Vec<Vector> {
    let input = include_str!("../../../../docs/transport/vectors/notify-v1.tsv");
    assert!(input.len() < 1_048_576);
    let rows: Vec<_> = input
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("category\t"))
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 6);
            let value = Vector {
                category: fields[0],
                name: fields[1],
                layer: fields[2],
                expectation: fields[3],
                bytes: hex(fields[4]),
                digest: hex(fields[5]).try_into().unwrap(),
            };
            assert!(
                !value.category.is_empty()
                    && !value.name.is_empty()
                    && !value.layer.is_empty()
                    && !value.expectation.is_empty()
            );
            assert_eq!(
                Sha256::digest(&value.bytes).as_slice(),
                value.digest,
                "fixture hash {}",
                value.name
            );
            value
        })
        .collect();
    assert!(rows.len() <= 256);
    rows
}
