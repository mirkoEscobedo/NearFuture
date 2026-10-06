use nf_contract::identity::*;
use nf_kernel::*;
mod support;
#[test]
fn leader_terms_and_sessions_fence_results_without_rerolling_strategy() {
    let world = support::fixture();
    let old = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let changed = AuthorityContext {
        term: AuthorityTerm(888),
        session: RuntimeSession(999),
    };
    let original = admit(&world, vec![support::peace_intent(&world, 1)], old).unwrap();
    let replacement = admit(&world, vec![support::peace_intent(&world, 1)], changed).unwrap();
    assert_eq!(
        evaluate(&original, JobId::from_bytes([1; 16]))
            .unwrap()
            .events,
        evaluate(&replacement, JobId::from_bytes([1; 16]))
            .unwrap()
            .events
    );
    let Settlement::Committed { world: a, .. } =
        settle(&world, &original, vec![], old, MissingPolicy::Recompute).unwrap()
    else {
        panic!("complete")
    };
    let Settlement::Committed { world: b, .. } = settle(
        &world,
        &replacement,
        vec![],
        changed,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("complete")
    };
    assert_eq!(state_hash(&a), state_hash(&b));
    assert_eq!(
        settle(&world, &original, vec![], changed, MissingPolicy::Recompute),
        Err(Rejection::FencedAuthority)
    );
    assert_eq!(
        settle(
            &world,
            &original,
            vec![],
            AuthorityContext {
                term: old.term,
                session: changed.session
            },
            MissingPolicy::Recompute
        ),
        Err(Rejection::StaleSession)
    );
    let old_proposal = evaluate(&original, JobId::from_bytes([1; 16])).unwrap();
    let Settlement::Committed { batch, .. } = settle(
        &world,
        &replacement,
        vec![JobResult {
            job: old_proposal.job,
            result: Ok(old_proposal),
        }],
        changed,
        MissingPolicy::Pause,
    )
    .unwrap() else {
        panic!("complete")
    };
    assert_eq!(batch.outcomes[0].rejection, Some(Rejection::StaleSession));
}
#[test]
fn results_outside_or_duplicate_exact_frontier_reject_before_mutation() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![support::peace_intent(&world, 1)], authority).unwrap();
    let result = JobResult {
        job: JobId::from_bytes([1; 16]),
        result: evaluate(&frontier, JobId::from_bytes([1; 16])),
    };
    assert_eq!(
        settle(
            &world,
            &frontier,
            vec![result.clone(), result],
            authority,
            MissingPolicy::Pause
        ),
        Err(Rejection::DuplicateJob)
    );
    assert_eq!(
        settle(
            &world,
            &frontier,
            vec![JobResult {
                job: JobId::from_bytes([99; 16]),
                result: Err(Rejection::ProviderFailed)
            }],
            authority,
            MissingPolicy::Pause
        ),
        Err(Rejection::UnknownJob)
    );
}
