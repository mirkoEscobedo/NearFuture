use nf_contract::identity::*;
use nf_kernel::*;
mod support;
#[test]
fn persisted_frontier_and_committed_events_restore_without_provider_execution() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let frontier = admit(
        &world,
        vec![
            support::peace_intent(&world, 1),
            support::market_intent(&world, 2, 7),
        ],
        authority,
    )
    .unwrap();
    let restored = decode_frontier(&encode_frontier(&frontier).unwrap()).unwrap();
    assert_eq!(restored, frontier);
    let Settlement::Committed { world: next, batch } = settle(
        &world,
        &restored,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("static jobs")
    };
    let persisted = decode_batch(&encode_batch(&batch).unwrap()).unwrap();
    assert_eq!(persisted, batch);
    assert_eq!(apply_batch(&world, &persisted), Ok(*next));
}
#[test]
fn replay_rejects_events_outside_exact_admitted_frontier_even_with_matching_hash() {
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
    assert_eq!(apply_batch(&world, &batch), Err(Rejection::InvalidProposal));
}
