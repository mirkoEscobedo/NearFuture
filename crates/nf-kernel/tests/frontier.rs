use nf_contract::identity::*;
use nf_kernel::*;
mod support;
#[test]
fn missing_exact_job_pauses_without_applying_arrival_winner() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
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
    let arrived = JobResult {
        job: JobId::from_bytes([2; 16]),
        result: evaluate(&frontier, JobId::from_bytes([2; 16])),
    };
    assert_eq!(
        settle(
            &world,
            &frontier,
            vec![arrived],
            authority,
            MissingPolicy::Pause
        ),
        Ok(Settlement::Paused(vec![JobId::from_bytes([1; 16])]))
    );
    assert_eq!(
        world.view().market(support::entity(30)).unwrap().credits,
        100
    );
}
#[test]
fn complete_frontier_commits_independent_commands_atomically_and_recomputes_missing() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
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
    let Settlement::Committed { world: next, batch } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("missing static jobs must recompute")
    };
    assert_eq!(
        next.view().market(support::entity(30)).unwrap().credits,
        107
    );
    assert_eq!(
        next.view().relation(support::entity(20)).unwrap().score,
        -495
    );
    assert_eq!(
        next.view().provider(support::provider(40)).unwrap().draws,
        1
    );
    assert_eq!(next.view().tick(), WorldTick(1));
    assert_eq!(next.view().event_seq(), EventSeq(1));
    assert_eq!(batch.outcomes.len(), 2);
    assert!(batch.outcomes.iter().all(|v| v.rejection.is_none()));
    assert_eq!(
        world.view().market(support::entity(30)).unwrap().credits,
        100
    );
}
