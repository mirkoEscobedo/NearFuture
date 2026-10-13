//! Caller-supplied composition images exercise the existing stale-frontier fence.
//! This is pure value coverage, not durable activation or migration acceptance.
use nf_contract::identity::{AuthorityTerm, EntityId, JobId, ProviderId, RuntimeSession};
use nf_kernel::{AuthorityContext, JobResult, MissingPolicy, Rejection, Settlement, World};
use nf_store::migration::{Composition, ProviderVersion};
mod support;

fn supplied_world(template: &World, ruleset: [u8; 32]) -> World {
    let mut spec = template.to_spec();
    spec.ruleset_hash = ruleset;
    for owner in &mut spec.ownership {
        owner.ruleset_hash = ruleset;
    }
    World::new(spec).unwrap()
}

#[test]
fn config_only_composition_change_fences_an_actually_evaluated_old_job() {
    let provider = ProviderVersion {
        provider: ProviderId::from_bytes([41; 16]),
        implementation: [41; 32],
        version: 1,
        state_schema: 1,
    };
    let support_set = [provider];
    let old_composition = Composition::new(1, [1; 32], [2; 32], [3; 32], vec![provider]).unwrap();
    let new_composition = Composition::new(1, [1; 32], [9; 32], [3; 32], vec![provider]).unwrap();
    assert_eq!(old_composition.require_supported(&support_set), Ok(()));
    assert_eq!(new_composition.require_supported(&support_set), Ok(()));
    assert_ne!(old_composition.digest(), new_composition.digest());

    let fixture = support::strategic_world();
    let old = supplied_world(&fixture, old_composition.digest());
    let current = supplied_world(&fixture, new_composition.digest());
    let old_before = old.clone();
    let current_before = current.clone();
    assert_eq!(old.to_spec().registry, current.to_spec().registry);
    assert_ne!(
        nf_kernel::state_hash(&old).unwrap(),
        nf_kernel::state_hash(&current).unwrap()
    );
    // Explicit test context, unchanged across both images; no authority is issued.
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(11),
    };
    let intent = support::intent(&old, 1, 5);
    let frontier = nf_kernel::admit(&old, vec![intent], authority).unwrap();
    assert_eq!(frontier.jobs().len(), 1);
    let proposal = nf_kernel::evaluate(&frontier, JobId::from_bytes([1; 16])).unwrap();
    let results = vec![JobResult {
        job: proposal.job,
        result: Ok(proposal),
    }];
    let Settlement::Committed { world, batch } = nf_kernel::settle(
        &old,
        &frontier,
        results.clone(),
        authority,
        MissingPolicy::Pause,
    )
    .unwrap() else {
        panic!("the genuine old job must settle on its original composition");
    };
    assert_eq!(
        world
            .view()
            .market(EntityId::from_bytes([30; 16]))
            .unwrap()
            .credits,
        105
    );
    assert_eq!(batch.outcomes.len(), 1);
    assert_eq!(batch.outcomes[0].rejection, None);
    assert_eq!(
        nf_kernel::settle(
            &current,
            &frontier,
            results,
            authority,
            MissingPolicy::Pause,
        ),
        Err(Rejection::StateMismatch)
    );
    assert_eq!(old, old_before);
    assert_eq!(current, current_before);
}
