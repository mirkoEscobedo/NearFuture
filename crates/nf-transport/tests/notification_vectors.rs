mod notification_support;
use nf_transport::notification::{NotifyLimits, PROTOCOL, decode_body, encode_body};
use std::collections::BTreeSet;

#[test]
fn independently_encoded_positive_records_cover_all_ten_closed_kinds() {
    let mut kinds = BTreeSet::new();
    let mut count = 0;
    for row in notification_support::vectors()
        .into_iter()
        .filter(|row| row.category == "record")
    {
        assert_eq!(row.layer, "shape");
        assert_eq!(row.expectation, "ADMIT_DATA");
        let record = decode_body(&row.bytes, PROTOCOL, NotifyLimits::default())
            .unwrap_or_else(|error| panic!("{}: {error:?}", row.name));
        assert_eq!(
            encode_body(&record, PROTOCOL, NotifyLimits::default()).unwrap(),
            row.bytes,
            "{}",
            row.name
        );
        kinds.insert(row.bytes[14]);
        count += 1;
    }
    assert_eq!(count, 16);
    assert_eq!(kinds, (1..=10).collect());
}

#[test]
fn shape_and_limit_negatives_reject_without_claiming_context_authorization() {
    let mut count = 0;
    for row in notification_support::vectors()
        .into_iter()
        .filter(|row| row.category == "negative" && matches!(row.layer, "shape" | "limit"))
    {
        assert!(row.expectation.starts_with("REJECT_"));
        assert!(
            decode_body(&row.bytes, PROTOCOL, NotifyLimits::default()).is_err(),
            "{}",
            row.name
        );
        count += 1;
    }
    assert_eq!(count, 86);
}

#[test]
fn finite_mutations_and_all_truncations_never_escape_bounded_closed_codec() {
    let rows = notification_support::vectors();
    let records: Vec<_> = rows.iter().filter(|row| row.category == "record").collect();
    for row in &records {
        for length in 0..row.bytes.len() {
            assert!(decode_body(&row.bytes[..length], PROTOCOL, NotifyLimits::default()).is_err());
        }
    }
    let mut random = 0x98a0_1234_u32;
    for n in 0..10_000 {
        random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let row = records[n % records.len()];
        let mut bytes = row.bytes.clone();
        let offset = (random as usize) % bytes.len();
        bytes[offset] ^= ((random >> 24) as u8) | 1;
        if n % 3 == 0 {
            bytes.truncate((random as usize) % bytes.len());
        }
        if n % 5 == 0 {
            bytes.extend([0; 8]);
        }
        assert!(bytes.len() <= 1024);
        if let Ok(record) = decode_body(&bytes, PROTOCOL, NotifyLimits::default()) {
            assert_eq!(
                encode_body(&record, PROTOCOL, NotifyLimits::default()).unwrap(),
                bytes
            );
        }
    }
}
