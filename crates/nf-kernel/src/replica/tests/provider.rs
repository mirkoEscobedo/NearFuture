use super::super::provider::validate_intent_with_scope;
use super::support;
use crate::Rejection;
use alloc::collections::BTreeMap;
use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ReplicaUsage, ScopedError,
};
use nf_contract::identity::AggregateRevision;

#[test]
fn resource_refusal_is_fatal_before_the_ordinary_expected_map_rejection() {
    let world = support::fixture();
    // Independent literal costs: set16 + read-map24 per read ID, two write-map24 rows.
    // A fresh logical map-entry admission costs24; moving an already-owned payload costs0.
    let scenarios = [
        (
            support::market_intent(&world, 1, 7),
            &[10, 30, 41][..],
            [30, 41],
            8,
            168,
            167,
        ),
        (
            support::peace_intent(&world, 2),
            &[10, 11, 20, 40][..],
            [20, 40],
            10,
            208,
            207,
        ),
    ];
    for (valid, read_ids, write_ids, entries, copied, low_copied) in scenarios {
        let mut invalid = valid.clone();
        assert!(invalid.expected.remove(&support::aggregate(10)).is_some());
        assert_eq!(
            crate::provider::validate_intent(&world, &invalid),
            Err(Rejection::InvalidProposal)
        );
        let low = ReplicaDecodeLimits::new(3, entries, low_copied).unwrap();
        let refused = low.with_scope::<_, (), _>(|scope| {
            let result = validate_intent_with_scope(&world, &invalid, scope);
            // Inspect the validator's outer Result before the sticky root can override it.
            assert_eq!(result, Err(ReplicaBudgetError::CopiedBytes));
            result.map_err(ScopedError::Budget)
        });
        assert_eq!(
            refused,
            Err(ScopedError::Budget(ReplicaBudgetError::CopiedBytes))
        );
        let ample = ReplicaDecodeLimits::new(3, entries, copied).unwrap();
        let (ordinary, usage) = ample
            .with_scope::<_, (), _>(|scope| {
                let result = validate_intent_with_scope(&world, &invalid, scope)
                    .map_err(ScopedError::Budget)?;
                Ok((result, scope.usage()))
            })
            .unwrap();
        assert_eq!(ordinary, Err(Rejection::InvalidProposal));
        assert_eq!(
            usage,
            ReplicaUsage {
                entries: u64::from(entries),
                copied_bytes: copied,
                depth: 1,
            }
        );
        let expected_reads: BTreeMap<_, _> = read_ids
            .iter()
            .map(|id| (support::aggregate(*id), AggregateRevision(0)))
            .collect();
        let expected_writes: BTreeMap<_, _> = write_ids
            .into_iter()
            .map(|id| (support::aggregate(id), AggregateRevision(0)))
            .collect();
        let expected = (expected_reads, expected_writes);
        let legacy = crate::provider::validate_intent(&world, &valid).unwrap();
        assert_eq!(legacy, expected);
        let (accepted, usage) = ample
            .with_scope::<_, (), _>(|scope| {
                let result = validate_intent_with_scope(&world, &valid, scope)
                    .map_err(ScopedError::Budget)?;
                Ok((result, scope.usage()))
            })
            .unwrap();
        assert_eq!(accepted, Ok(legacy));
        assert_eq!(usage.entries, u64::from(entries));
        assert_eq!(usage.copied_bytes, copied);
        assert_eq!(usage.depth, 1);
    }
}
