pub fn row(category: &str, name: &str) -> Vec<u8> {
    let line = include_str!("../../../../docs/transport/vectors/receipt-v2.tsv")
        .lines()
        .find(|line| {
            let mut fields = line.split('\t');
            fields.next() == Some(category) && fields.next() == Some(name)
        })
        .unwrap();
    decode(line.split('\t').nth(4).unwrap())
}
fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
