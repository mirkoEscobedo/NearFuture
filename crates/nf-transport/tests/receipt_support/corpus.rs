pub fn row(category: &str, name: &str) -> Vec<u8> {
    let line = include_str!("../../../../docs/transport/vectors/receipt-v2.tsv")
        .lines()
        .find(|line| {
            line.split('\t').next() == Some(category) && line.split('\t').nth(1) == Some(name)
        })
        .expect("independent row");
    let hex = line.split('\t').nth(4).unwrap();
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
pub fn matching(category: &str, layer: &str) -> Vec<(&'static str, Vec<u8>)> {
    include_str!("../../../../docs/transport/vectors/receipt-v2.tsv")
        .lines()
        .filter_map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len() != 6 || columns[0] != category || columns[2] != layer {
                return None;
            }
            Some((columns[1], row(category, columns[1])))
        })
        .collect()
}
