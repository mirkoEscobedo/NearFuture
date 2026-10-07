use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ReplicaUsage, ScopedError,
};
use nf_contract::identity::{AuthorityTerm, RuntimeSession};
use nf_kernel::replica::admit_with_scope;
use nf_kernel::{AuthorityContext, Rejection, admit, evaluate};
mod support;

fn authority() -> AuthorityContext {
    AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    }
}

#[test]
fn a_zero_copy_quota_is_fatal_before_a_stored_ordinary_validation_error() {
    let world = support::fixture();
    let mut invalid = support::market_intent(&world, 1, 7);
    assert!(invalid.expected.remove(&support::aggregate(10)).is_some());
    let inputs = [invalid];
    let legacy = admit(&world, inputs.to_vec(), authority()).unwrap();
    assert_eq!(
        evaluate(&legacy, inputs[0].job),
        Err(Rejection::InvalidProposal)
    );
    let refused = ReplicaDecodeLimits::new(7, 256, 0)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let direct = admit_with_scope(&world, &inputs, authority(), scope);
            // Inspect admission itself before sticky root completion can mask its result.
            assert_eq!(direct, Err(ReplicaBudgetError::CopiedBytes));
            direct.map_err(ScopedError::Budget)
        });
    assert_eq!(
        refused,
        Err(ScopedError::Budget(ReplicaBudgetError::CopiedBytes))
    );
}

#[test]
fn valid_public_admission_matches_legacy_at_the_proposed_exact_fit() {
    let world = support::fixture();
    let inputs = [support::market_intent(&world, 1, 7)];
    let expected = admit(&world, inputs.to_vec(), authority()).unwrap();
    // New admission arithmetic is proposed pending independent review: C2805/E92/D7.
    ReplicaDecodeLimits::new(7, 92, 2805)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let direct = admit_with_scope(&world, &inputs, authority(), scope)
                .map_err(ScopedError::Budget)?;
            assert_eq!(direct, Ok(expected));
            assert_eq!(
                scope.usage(),
                ReplicaUsage {
                    entries: 92,
                    copied_bytes: 2805,
                    depth: 1,
                }
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn sequential_public_admissions_share_the_same_cumulative_copy_quota() {
    let world = support::fixture();
    let inputs = [support::market_intent(&world, 1, 7)];
    let expected = admit(&world, inputs.to_vec(), authority()).unwrap();
    let refused = ReplicaDecodeLimits::new(7, 184, 5609)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let first = admit_with_scope(&world, &inputs, authority(), scope)
                .map_err(ScopedError::Budget)?;
            assert_eq!(first, Ok(expected));
            let second = admit_with_scope(&world, &inputs, authority(), scope);
            assert_eq!(second, Err(ReplicaBudgetError::CopiedBytes));
            second.map_err(ScopedError::Budget)
        });
    assert_eq!(
        refused,
        Err(ScopedError::Budget(ReplicaBudgetError::CopiedBytes))
    );
}
