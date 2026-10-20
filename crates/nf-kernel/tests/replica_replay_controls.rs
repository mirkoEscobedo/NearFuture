use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ScopedError,
};
use nf_contract::identity::{
    AggregateRevision, AuthorityTerm, EventSeq, RuntimeSession, WorldTick,
};
use nf_kernel::replica::apply_batch_with_scope;
use nf_kernel::*;
mod support;

fn committed_market() -> (World, World, CommittedBatch) {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let frontier = admit(
        &world,
        vec![support::market_intent(&world, 1, 7)],
        authority,
    )
    .unwrap();
    let Settlement::Committed {
        world: expected,
        batch,
    } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap()
    else {
        panic!("genuine static market settlement");
    };
    (world, *expected, batch)
}

#[test]
fn exact_entry_copy_and_depth_limits_match_settlement_and_default_replay() {
    let (world, expected, batch) = committed_market();
    assert_eq!(apply_batch(&world, &batch), Ok(expected.clone()));
    // Independent fixture accounting: two snapshot hashes, admission, two world
    // copies, retained access keys and one outcome cost E251/C7591, maximum D8.
    let actual = ReplicaDecodeLimits::new(8, 251, 7591)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            apply_batch_with_scope(&world, &batch, scope).map_err(ScopedError::Budget)
        });
    assert_eq!(actual, Ok(Ok(expected)));
}

#[test]
fn insufficient_entry_and_depth_limits_remain_fatal_after_caught_refusals() {
    let (world, _, batch) = committed_market();
    for (depth, entries, reason) in [
        (8, 250, ReplicaBudgetError::Entries),
        (7, 251, ReplicaBudgetError::Depth),
    ] {
        let result = ReplicaDecodeLimits::new(depth, entries, 7591)
            .unwrap()
            .with_scope::<_, (), _>(|scope| {
                assert_eq!(apply_batch_with_scope(&world, &batch, scope), Err(reason));
                let refused_usage = scope.usage();
                assert_eq!(apply_batch_with_scope(&world, &batch, scope), Err(reason));
                assert_eq!(scope.usage(), refused_usage);
                Ok(())
            });
        assert_eq!(result, Err(ScopedError::Budget(reason)));
    }
}

#[test]
fn entry_budget_is_cumulative_across_replays_in_one_root() {
    let (world, expected, batch) = committed_market();
    let result = ReplicaDecodeLimits::new(8, 501, 15182)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            assert_eq!(
                apply_batch_with_scope(&world, &batch, scope),
                Ok(Ok(expected))
            );
            assert_eq!(
                apply_batch_with_scope(&world, &batch, scope),
                Err(ReplicaBudgetError::Entries)
            );
            // Catching the refusal cannot convert the root operation to success.
            Ok(())
        });
    assert_eq!(
        result,
        Err(ScopedError::Budget(ReplicaBudgetError::Entries))
    );
}

#[test]
fn scoped_replay_refuses_malformed_hashes_clock_and_frontier_shapes() {
    let (world, _, batch) = committed_market();
    let mut cases = Vec::new();
    let mut before = batch.clone();
    before.before_hash[0] ^= 1;
    cases.push((before, Rejection::StateMismatch));
    let mut after = batch.clone();
    after.after_hash[0] ^= 1;
    cases.push((after, Rejection::StateMismatch));
    let mut tick = batch.clone();
    tick.tick = WorldTick(2);
    cases.push((tick, Rejection::StateMismatch));
    let mut sequence = batch.clone();
    sequence.sequence = EventSeq(2);
    cases.push((sequence, Rejection::StateMismatch));
    let mut outcomes = batch.clone();
    outcomes.outcomes.clear();
    cases.push((outcomes, Rejection::InvalidFrontier));
    let mut truncated = batch.clone();
    truncated.events.pop();
    cases.push((truncated, Rejection::InvalidProposal));
    let mut trailing = batch.clone();
    trailing.events.push(batch.events[0].clone());
    cases.push((trailing, Rejection::InvalidProposal));
    let retained = world.clone();
    for (malformed, expected) in cases {
        let result = ReplicaDecodeLimits::new(32, 32768, 4194304)
            .unwrap()
            .with_scope::<_, (), _>(|scope| {
                assert_eq!(
                    apply_batch_with_scope(&world, &malformed, scope),
                    Ok(Err(expected))
                );
                assert_eq!(scope.failure(), None);
                Ok(())
            });
        assert_eq!(result, Ok(()));
        assert_eq!(world, retained);
    }
}

#[test]
fn scoped_replay_refuses_unadmitted_events_even_with_a_matching_after_hash() {
    let world = support::fixture();
    let mut forged = world.to_spec();
    forged.markets[0].credits = 107;
    forged
        .revisions
        .insert(support::aggregate(30), AggregateRevision(1));
    forged.tick = WorldTick(1);
    forged.event_seq = EventSeq(1);
    let forged = World::new(forged).unwrap();
    let batch = CommittedBatch {
        before_hash: state_hash(&world).unwrap(),
        after_hash: state_hash(&forged).unwrap(),
        tick: WorldTick(1),
        sequence: EventSeq(1),
        authority: AuthorityContext {
            term: AuthorityTerm(1),
            session: RuntimeSession(1),
        },
        admitted: vec![],
        outcomes: vec![],
        events: vec![Event::MarketAdjusted {
            market: support::entity(30),
            delta: 7,
        }],
    };
    let result = ReplicaDecodeLimits::new(32, 32768, 4194304)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            assert_eq!(
                apply_batch_with_scope(&world, &batch, scope),
                Ok(Err(Rejection::InvalidProposal))
            );
            Ok(())
        });
    assert_eq!(result, Ok(()));
}
