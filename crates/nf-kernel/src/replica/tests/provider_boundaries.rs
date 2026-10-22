use super::super::provider::validate_intent_with_scope;
use super::support;
use crate::{Intent, Rejection, World};
use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ScopedError,
};

#[test]
fn entries_are_fatal_before_the_expected_map_rejection() {
    let world = support::fixture();
    // Independent exact costs are market C168/E8 and relation C208/E10.
    for (valid, limits) in [
        (
            support::market_intent(&world, 1, 7),
            ReplicaDecodeLimits::new(3, 7, 168).unwrap(),
        ),
        (
            support::peace_intent(&world, 2),
            ReplicaDecodeLimits::new(3, 9, 208).unwrap(),
        ),
    ] {
        assert_refusal(&world, &valid, limits, ReplicaBudgetError::Entries);
    }
}

#[test]
fn depth_is_fatal_before_the_expected_map_rejection() {
    let world = support::fixture();
    // The private validator descends root1 -> collection2 -> admitted row3.
    for (valid, limits) in [
        (
            support::market_intent(&world, 1, 7),
            ReplicaDecodeLimits::new(2, 8, 168).unwrap(),
        ),
        (
            support::peace_intent(&world, 2),
            ReplicaDecodeLimits::new(2, 10, 208).unwrap(),
        ),
    ] {
        assert_refusal(&world, &valid, limits, ReplicaBudgetError::Depth);
    }
}

fn assert_refusal(
    world: &World,
    valid: &Intent,
    limits: ReplicaDecodeLimits,
    category: ReplicaBudgetError,
) {
    let mut invalid = valid.clone();
    assert!(invalid.expected.remove(&support::aggregate(10)).is_some());
    assert_eq!(
        crate::provider::validate_intent(world, &invalid),
        Err(Rejection::InvalidProposal)
    );
    let refused = limits.with_scope::<_, (), _>(|scope| {
        let direct = validate_intent_with_scope(world, &invalid, scope);
        // Inspect the validator itself before the sticky root can mask a wrong result.
        assert_eq!(direct, Err(category));
        direct.map_err(ScopedError::Budget)
    });
    assert_eq!(refused, Err(ScopedError::Budget(category)));
}
