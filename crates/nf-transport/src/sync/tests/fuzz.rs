use super::{super::*, support::*};
#[test]
fn finite_raw_mutations_never_escape_hard_or_selected_shape_admission() {
    let positives: Vec<_> = rows()
        .into_iter()
        .filter(|r| r.category() == "record")
        .collect();
    let mut random = 7u64;
    for n in 0..10000 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let r = &positives[n % positives.len()];
        let mut bytes = r.bytes();
        let i = (random as usize) % bytes.len();
        bytes[i] ^= (random >> 32) as u8;
        if let Ok(record) = decode_body(&bytes, r.policy()) {
            assert!(bytes.len() <= r.policy().frame_maximum().unwrap());
            assert_eq!(encode_body(&record, r.policy()).unwrap(), bytes);
            if let SyncBody::SyncChunk(v) = record.body {
                assert!(v.data.len() <= r.policy().limits.chunk_bytes as usize);
            }
        }
    }
}
