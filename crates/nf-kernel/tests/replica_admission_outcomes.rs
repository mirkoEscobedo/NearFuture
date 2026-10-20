use nf_contract::canonical::replica_budget::{ReplicaDecodeLimits, ScopedError};
use nf_contract::identity::{AuthorityTerm, RuntimeSession};
use nf_kernel::replica::admit_with_scope;
use nf_kernel::{AuthorityContext, MissingPolicy, Rejection, Settlement, admit, evaluate, settle};
mod support;

#[test]
fn public_admission_matches_legacy_on_real_successful_and_rejected_outcomes() {
    let genesis = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let mut rejected = support::market_intent(&genesis, 2, 7);
    assert!(rejected.expected.remove(&support::aggregate(10)).is_some());
    let frontier = admit(
        &genesis,
        vec![support::peace_intent(&genesis, 1), rejected],
        authority,
    )
    .unwrap();
    let Settlement::Committed { world, batch } = settle(
        &genesis,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("real retained outcomes required")
    };
    assert_eq!(batch.outcomes.len(), 2);
    assert!(
        world
            .outcomes()
            .iter()
            .any(|value| value.rejection.is_none())
    );
    assert!(
        world
            .outcomes()
            .iter()
            .any(|value| value.rejection == Some(Rejection::InvalidProposal))
    );
    let input = support::market_intent(&world, 3, 7);
    let inputs = [input];
    let expected = admit(&world, inputs.to_vec(), authority).unwrap();
    let actual = ReplicaDecodeLimits::new(7, 16384, 1_048_576)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            admit_with_scope(&world, &inputs, authority, scope).map_err(ScopedError::Budget)
        })
        .unwrap()
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.snapshot_world().outcomes(), world.outcomes());
    assert_eq!(actual.input_hash(), nf_kernel::state_hash(&world).unwrap());
    assert!(evaluate(&actual, inputs[0].job).is_ok());
}
