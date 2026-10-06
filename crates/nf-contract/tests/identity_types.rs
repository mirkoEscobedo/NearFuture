use nf_contract::identity::{AccountId, HistoryId, RequestId};

#[test]
fn opaque_identity_bytes_roundtrip_without_text_normalization() {
    let bytes = [0x81; 16];
    assert_eq!(AccountId::from_bytes(bytes).as_bytes(), &bytes);
    assert_eq!(HistoryId::from_slice(&bytes).unwrap().as_bytes(), &bytes);
    assert!(RequestId::from_slice(&bytes[..15]).is_err());
}

#[test]
fn distinct_counters_refuse_overflow_instead_of_wrapping() {
    use nf_contract::identity::{
        AggregateRevision, AuthorityTerm, EventSeq, RulesetRevision, RuntimeSession, WorldTick,
    };
    assert!(AuthorityTerm(u64::MAX).checked_next().is_none());
    assert!(RuntimeSession(u64::MAX).checked_next().is_none());
    assert!(RulesetRevision(u64::MAX).checked_next().is_none());
    assert!(WorldTick(u64::MAX).checked_next().is_none());
    assert!(EventSeq(u64::MAX).checked_next().is_none());
    assert!(AggregateRevision(u64::MAX).checked_next().is_none());
    assert_eq!(WorldTick(0).checked_next(), Some(WorldTick(1)));
}
