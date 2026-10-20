use nf_contract::canonical::replica_budget::{ReplicaDecodeLimits, ReplicaUsage, ScopedError};
use nf_contract::identity::{AccountId, AggregateRevision, AuthorityTerm, RuntimeSession};
use nf_kernel::replica::admit_with_scope;
use nf_kernel::{
    AuthorityContext, Frontier, Intent, MAX_ENTITIES, MAX_FRONTIER, MAX_PROVIDERS, MissingPolicy,
    Rejection, Settlement, World, admit, evaluate, settle,
};
mod support;

fn authority() -> AuthorityContext {
    AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    }
}
fn scoped(world: &World, inputs: &[Intent]) -> Result<Frontier, Rejection> {
    ReplicaDecodeLimits::new(7, 16384, 1_048_576)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            admit_with_scope(world, inputs, authority(), scope).map_err(ScopedError::Budget)
        })
        .unwrap()
}

#[test]
fn multiple_admission_jobs_match_the_actual_legacy_total_order_and_maps() {
    let world = support::fixture();
    let inputs = [
        support::market_intent(&world, 3, 3),
        support::peace_intent(&world, 2),
        support::market_intent(&world, 1, 7),
    ];
    let expected = admit(&world, inputs.to_vec(), authority()).unwrap();
    let actual = scoped(&world, &inputs).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        actual.jobs().iter().map(|job| job.id()).collect::<Vec<_>>(),
        vec![inputs[1].job, inputs[2].job, inputs[0].job]
    );
}

#[test]
fn ordinary_invalid_stale_and_unauthorized_validation_remain_stored() {
    let world = support::fixture();
    let mut invalid = support::market_intent(&world, 1, 7);
    invalid.expected.remove(&support::aggregate(10)).unwrap();
    let mut stale = support::market_intent(&world, 2, 7);
    stale
        .expected
        .insert(support::aggregate(10), AggregateRevision(1));
    let mut unauthorized = support::market_intent(&world, 3, 7);
    unauthorized.actor = AccountId::from_bytes([91; 16]);
    for (intent, reason) in [
        (invalid, Rejection::InvalidProposal),
        (stale, Rejection::StaleRevision),
        (unauthorized, Rejection::Unauthorized),
    ] {
        let expected = admit(&world, vec![intent.clone()], authority()).unwrap();
        let actual = scoped(&world, &[intent]);
        assert_eq!(actual, Ok(expected.clone()));
        assert_eq!(evaluate(&expected, expected.jobs()[0].id()), Err(reason));
    }
}

#[test]
fn duplicate_job_operation_and_request_keep_legacy_rejections() {
    let world = support::fixture();
    let original = support::market_intent(&world, 1, 7);
    for field in 0..3 {
        let mut duplicate = support::market_intent(&world, 2, 7);
        match field {
            0 => duplicate.job = original.job,
            1 => duplicate.operation = original.operation,
            _ => duplicate.request = original.request,
        }
        let inputs = [original.clone(), duplicate];
        assert_eq!(
            admit(&world, inputs.to_vec(), authority()),
            Err(Rejection::InvalidFrontier)
        );
        assert_eq!(scoped(&world, &inputs), Err(Rejection::InvalidFrontier));
    }
}

#[test]
fn durable_actual_outcome_collisions_keep_legacy_rejection() {
    let genesis = support::fixture();
    let intent = support::peace_intent(&genesis, 1);
    let frontier = admit(&genesis, vec![intent.clone()], authority()).unwrap();
    let Settlement::Committed { world, batch } = settle(
        &genesis,
        &frontier,
        vec![],
        authority(),
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("real retained outcome required")
    };
    assert_eq!(batch.outcomes.len(), 1);
    assert!(batch.outcomes[0].rejection.is_none());
    let inputs = [intent];
    assert_eq!(
        admit(&world, inputs.to_vec(), authority()),
        Err(Rejection::InvalidFrontier)
    );
    assert_eq!(scoped(&world, &inputs), Err(Rejection::InvalidFrontier));
}

#[test]
fn maximum_frontier_local_cap_is_preserved_before_any_new_owner_charge() {
    let world = support::fixture();
    let inputs = vec![support::market_intent(&world, 1, 7); MAX_FRONTIER + 1];
    assert_eq!(
        admit(&world, inputs.clone(), authority()),
        Err(Rejection::Limit)
    );
    ReplicaDecodeLimits::new(1, 0, 0)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let direct = admit_with_scope(&world, &inputs, authority(), scope)
                .map_err(ScopedError::Budget)?;
            assert_eq!(direct, Err(Rejection::Limit));
            assert_eq!(
                scope.usage(),
                ReplicaUsage {
                    entries: 0,
                    copied_bytes: 0,
                    depth: 1
                }
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn per_intent_expected_local_cap_precedes_that_rows_duplicate_check() {
    let world = support::fixture();
    let first = support::market_intent(&world, 1, 7);
    let mut second = first.clone();
    second.expected.clear();
    for id in 0..=(MAX_ENTITIES + MAX_PROVIDERS) {
        second.expected.insert(
            support::aggregate(u8::try_from(id).unwrap()),
            AggregateRevision(0),
        );
    }
    let inputs = [first, second];
    assert_eq!(
        admit(&world, inputs.to_vec(), authority()),
        Err(Rejection::Limit)
    );
    assert_eq!(scoped(&world, &inputs), Err(Rejection::Limit));
}

#[test]
fn empty_public_admission_keeps_the_canonical_snapshot_hash() {
    let world = support::fixture();
    assert_eq!(scoped(&world, &[]), admit(&world, vec![], authority()));
}
