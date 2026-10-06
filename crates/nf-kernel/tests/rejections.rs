use nf_contract::identity::*;
use nf_kernel::*;
mod support;
#[test]
fn unauthorized_write_set_is_recorded_without_any_mutation() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![support::peace_intent(&world, 1)], authority).unwrap();
    let mut proposal = evaluate(&frontier, JobId::from_bytes([1; 16])).unwrap();
    proposal
        .writes
        .insert(support::aggregate(10), AggregateRevision(0));
    let Settlement::Committed { world: next, batch } = settle(
        &world,
        &frontier,
        vec![JobResult {
            job: proposal.job,
            result: Ok(proposal),
        }],
        authority,
        MissingPolicy::Pause,
    )
    .unwrap() else {
        panic!("complete")
    };
    assert_eq!(batch.outcomes[0].rejection, Some(Rejection::Unauthorized));
    assert!(batch.events.is_empty());
    assert_eq!(
        next.view().relation(support::entity(20)).unwrap().score,
        -500
    );
    assert_eq!(
        next.view().provider(support::provider(40)).unwrap().draws,
        0
    );
}
#[test]
fn stale_proposal_read_revision_records_specific_reason() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![support::peace_intent(&world, 1)], authority).unwrap();
    let mut proposal = evaluate(&frontier, JobId::from_bytes([1; 16])).unwrap();
    proposal
        .reads
        .insert(support::aggregate(20), AggregateRevision(99));
    let Settlement::Committed { batch, .. } = settle(
        &world,
        &frontier,
        vec![JobResult {
            job: proposal.job,
            result: Ok(proposal),
        }],
        authority,
        MissingPolicy::Pause,
    )
    .unwrap() else {
        panic!("complete")
    };
    assert_eq!(batch.outcomes[0].rejection, Some(Rejection::StaleRevision));
    assert!(batch.events.is_empty());
}
#[test]
fn provider_draw_overflow_is_preserved_as_deterministic_rejection() {
    let mut spec = support::fixture().to_spec();
    spec.providers[0].draws = u64::MAX;
    let world = World::new(spec).unwrap();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![support::peace_intent(&world, 1)], authority).unwrap();
    let Settlement::Committed { world: next, batch } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("recompute")
    };
    assert_eq!(batch.outcomes[0].rejection, Some(Rejection::Overflow));
    assert!(batch.events.is_empty());
    assert_eq!(
        next.view().relation(support::entity(20)).unwrap().score,
        -500
    );
}
#[test]
fn revision_overflow_rolls_back_target_mutation_and_provider_state_together() {
    let mut spec = support::fixture().to_spec();
    spec.revisions
        .insert(support::aggregate(41), AggregateRevision(u64::MAX));
    let world = World::new(spec).unwrap();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(
        &world,
        vec![support::market_intent(&world, 1, 7)],
        authority,
    )
    .unwrap();
    let Settlement::Committed { world: next, batch } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("recompute")
    };
    assert_eq!(batch.outcomes[0].rejection, Some(Rejection::Overflow));
    assert!(batch.events.is_empty());
    assert_eq!(
        next.view().market(support::entity(30)).unwrap().credits,
        100
    );
    assert_eq!(
        next.view()
            .provider(support::provider(41))
            .unwrap()
            .cooldown_until,
        WorldTick(0)
    );
}
#[test]
fn provider_failure_and_stale_or_unauthorized_intents_have_durable_reasons() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    for (index, expected) in [
        Rejection::ProviderFailed,
        Rejection::Unauthorized,
        Rejection::StaleRevision,
    ]
    .into_iter()
    .enumerate()
    {
        let mut intent = support::peace_intent(&world, 1);
        if index == 1 {
            intent.actor = AccountId::from_bytes([99; 16]);
        }
        if index == 2 {
            intent
                .expected
                .insert(support::aggregate(20), AggregateRevision(88));
        }
        let frontier = admit(&world, vec![intent], authority).unwrap();
        let Settlement::Committed { world: next, batch } = settle(
            &world,
            &frontier,
            vec![JobResult {
                job: JobId::from_bytes([1; 16]),
                result: Err(Rejection::ProviderFailed),
            }],
            authority,
            MissingPolicy::Pause,
        )
        .unwrap() else {
            panic!("complete")
        };
        assert_eq!(batch.outcomes[0].rejection, Some(expected));
        assert!(batch.events.is_empty());
        assert_eq!(
            decode_snapshot(&encode_snapshot(&next).unwrap())
                .unwrap()
                .outcomes(),
            next.outcomes()
        );
        assert_eq!(apply_batch(&world, &batch), Ok(*next));
    }
}
