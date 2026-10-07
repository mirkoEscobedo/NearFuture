use nf_transport::receipt_effects::book::{decode_book, encode_book};
fn row(name: &str) -> Vec<u8> {
    let line = include_str!("../../../docs/transport/vectors/receipt-v2.tsv")
        .lines()
        .find(|line| {
            let mut f = line.split('\t');
            f.next() == Some("book") && f.next() == Some(name)
        })
        .unwrap();
    let hex = line.split('\t').nth(4).unwrap().as_bytes();
    hex.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn independent_book_records_round_trip_and_commit_the_original_payload() {
    for generation in 0..8 {
        let raw = row(&format!("generation-{generation}"));
        let record = decode_book(&raw).unwrap();
        assert_eq!(encode_book(&record).unwrap().as_slice(), raw);
    }
    let mut changed = row("generation-0");
    changed[412] ^= 1;
    assert!(
        decode_book(&changed).is_err(),
        "retained full payload/binding mismatch admitted"
    );
}
