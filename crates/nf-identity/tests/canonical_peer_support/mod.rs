use sha2::{Digest, Sha256};

pub(super) fn corpus(name: &str) -> Vec<u8> {
    let row = include_str!("../../../../docs/transport/vectors/receipt-v2.tsv")
        .lines()
        .find(|line| line.split('\t').nth(1) == Some(name))
        .expect("independent fixture exists");
    let fields: Vec<_> = row.split('\t').collect();
    let bytes: Vec<_> = fields[4]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(digest, fields[5], "independent fixture identity");
    bytes
}
