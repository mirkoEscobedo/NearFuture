use nf_contract::identity::*;
use nf_kernel::*;
mod support;
#[test]
fn overlapping_jobs_choose_stable_order_and_record_conflict_independent_of_arrival() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(
        &world,
        vec![
            support::peace_intent(&world, 2),
            support::peace_intent(&world, 1),
        ],
        authority,
    )
    .unwrap();
    let mut results: Vec<_> = frontier
        .jobs()
        .iter()
        .map(|job| JobResult {
            job: job.id(),
            result: evaluate(&frontier, job.id()),
        })
        .collect();
    let expected = settle(
        &world,
        &frontier,
        results.clone(),
        authority,
        MissingPolicy::Pause,
    )
    .unwrap();
    results.reverse();
    assert_eq!(
        settle(&world, &frontier, results, authority, MissingPolicy::Pause),
        Ok(expected.clone())
    );
    let Settlement::Committed { world: next, batch } = expected else {
        panic!("alljobs present")
    };
    assert_eq!(
        next.view().relation(support::entity(20)).unwrap().score,
        -495
    );
    assert_eq!(batch.outcomes[0].rejection, None);
    assert_eq!(batch.outcomes[1].rejection, Some(Rejection::Conflict));
    assert_eq!(
        next.view().provider(support::provider(40)).unwrap().draws,
        1
    );
}
