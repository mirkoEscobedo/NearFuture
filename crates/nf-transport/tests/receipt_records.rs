#[path = "receipt_support/corpus.rs"]
mod corpus;
use nf_transport::{
    receipt::{decode_body, encode_body},
    records::PeerLimits,
};
#[test]
fn all_registered_receipt_records_admit_without_granting_authority() {
    let rows = corpus::matching("record", "shape");
    assert_eq!(rows.len(), 14, "independent registry corpus count");
    for (name, raw) in rows {
        let r = decode_body(&raw, PeerLimits::default())
            .unwrap_or_else(|e| panic!("registered data {name}: {e:?}"));
        assert_eq!(
            encode_body(&r, PeerLimits::default()).unwrap(),
            raw,
            "independent exact bytes {name}"
        );
    }
}
#[test]
fn malformed_receipt_shape_and_limits_are_closed() {
    for layer in ["shape", "limit"] {
        for (name, raw) in corpus::matching("negative", layer) {
            assert!(
                decode_body(&raw, PeerLimits::default()).is_err(),
                "malformed {name} admitted"
            );
        }
    }
}
