use super::{super::*, support::*};
#[test]
fn all_independent_record_and_prefix_bytes_roundtrip_exactly() {
    let mut count = 0;
    let mut prefixes = 0;
    let mut kinds = [false; 17];
    for r in rows().into_iter().filter(|r| r.category() == "record") {
        let raw = r.bytes();
        assert_eq!(hash(&raw).to_vec(), hex(r.fields[5]), "{}", r.name());
        let record = decode_body(&raw, r.policy()).unwrap_or_else(|e| panic!("{} {e:?}", r.name()));
        assert_eq!(
            encode_body(&record, r.policy()).unwrap(),
            raw,
            "{}",
            r.name()
        );
        assert_eq!(
            decode_frame(&encode_frame(&record, r.policy()).unwrap(), r.policy()).unwrap(),
            record
        );
        kinds[usize::from(record.body.kind()) - 1] = true;
        count += 1;
        if matches!(record.body.kind(), 2..=4 | 8 | 9 | 13 | 15 | 16 | 17) {
            let expected = row(&format!("{}-prefix", r.name())).bytes();
            assert_eq!(&raw[..raw.len() - 297], expected);
            prefixes += 1;
        }
        if matches!(record.body.kind(), 8 | 9 | 13 | 15 | 16 | 17) {
            let expected = row(&format!("{}-prefix", r.name())).bytes();
            assert_eq!(signed_prefix(&record, r.policy()).unwrap(), expected);
            assert_eq!(
                signed_prefix_digest(&record, r.policy()).unwrap(),
                hash(&expected)
            );
        }
    }
    assert_eq!(count, 25);
    assert_eq!(prefixes, 12);
    assert!(kinds.into_iter().all(|v| v));
}
#[test]
fn all_shape_profile_framing_negatives_fail_before_domain_use() {
    let mut count = 0;
    for r in rows()
        .into_iter()
        .filter(|r| r.category() == "malformed" && r.layer() != "context")
    {
        let result = if r.layer() == "framing" {
            let lane = if r.name().starts_with("control") {
                SyncLane::Control
            } else {
                SyncLane::Transfer
            };
            decode_frame(&r.bytes(), SyncWirePolicy { lane, ..r.policy() })
        } else {
            decode_body(&r.bytes(), r.policy())
        };
        assert!(result.is_err(), "{} admitted", r.name());
        if r.layer() == "framing" {
            assert!(
                matches!(result, Err(crate::PeerError::Limit)),
                "{}",
                r.name()
            );
        }
        count += 1;
    }
    assert_eq!(count, 68);
}
#[test]
fn independent_public_proof_primitives_verify_without_granting_authority() {
    let mut count = 0;
    for r in rows().into_iter().filter(|r| r.category() == "signature") {
        let name = r.name().strip_suffix("-signature").unwrap();
        let record_row = row(name);
        let p = proof(&record_row);
        let digest = nf_identity::signing::device_digest(&p).unwrap();
        nf_contract::signatures::verify_digest(
            &key(p.account.as_bytes()[0] == 1),
            &digest,
            &p.signature,
        )
        .unwrap();
        assert_eq!(p.signature.as_slice(), r.bytes());
        count += 1;
    }
    assert_eq!(count, 14);
}
