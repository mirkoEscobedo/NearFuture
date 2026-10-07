use super::{super::*, support::*};
#[test]
fn fixed_context_bytes_are_data_until_a_later_owner_supplies_expectations() {
    let mut count = 0;
    for r in rows().into_iter().filter(|r| r.layer() == "context") {
        if r.name() == "lane" {
            assert!(matches!(
                decode_body(
                    &r.bytes(),
                    SyncWirePolicy {
                        lane: SyncLane::Control,
                        ..r.policy()
                    }
                ),
                Err(crate::PeerError::Scope)
            ));
        } else {
            // Opaque digest/session/proof fields cannot be an admission of current policy,
            // descriptor custody or assembly merely because their fixed shapes parse.
            let record = decode_body(&r.bytes(), r.policy()).unwrap();
            assert_eq!(encode_body(&record, r.policy()).unwrap(), r.bytes());
        }
        count += 1;
    }
    assert_eq!(count, 7);
}
