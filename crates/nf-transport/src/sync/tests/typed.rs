use super::{super::*, support::*};

#[test]
fn all_17_typed_values_encode_exact_independent_record_bytes() {
    let mut count = 0;
    for r in rows().into_iter().filter(|r| r.category() == "record") {
        let lane = r.policy().lane;
        let kind = r.number(6, 0) as u8;
        let body = super::typed_control::body(&r, kind);
        let record = SyncRecord {
            lane,
            context: context(lane, kind == 1),
            body,
        };
        assert_eq!(
            encode_body(&record, r.policy()).unwrap(),
            r.bytes(),
            "{}",
            r.name()
        );
        assert_eq!(
            decode_body(&r.bytes(), r.policy()).unwrap(),
            record,
            "{}",
            r.name()
        );
        count += 1;
    }
    assert_eq!(count, 25);
}
