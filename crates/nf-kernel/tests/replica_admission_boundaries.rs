use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ReplicaUsage, ScopedError,
};
use nf_contract::identity::{AuthorityTerm, RuntimeSession};
use nf_kernel::replica::admit_with_scope;
use nf_kernel::{AuthorityContext, Intent, Rejection, World, admit, evaluate};
mod support;

fn authority() -> AuthorityContext {
    AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    }
}
fn malformed(world: &World) -> [Intent; 1] {
    let mut intent = support::market_intent(world, 1, 7);
    assert!(intent.expected.remove(&support::aggregate(10)).is_some());
    [intent]
}

#[test]
fn exact_quota_preserves_the_stored_late_ordinary_validation_error() {
    let world = support::fixture();
    let inputs = malformed(&world);
    let expected = admit(&world, inputs.to_vec(), authority()).unwrap();
    assert_eq!(
        evaluate(&expected, inputs[0].job),
        Err(Rejection::InvalidProposal)
    );
    ReplicaDecodeLimits::new(7, 91, 2781)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let direct = admit_with_scope(&world, &inputs, authority(), scope)
                .map_err(ScopedError::Budget)?;
            assert_eq!(direct, Ok(expected));
            assert_eq!(
                scope.usage(),
                ReplicaUsage {
                    entries: 91,
                    copied_bytes: 2781,
                    depth: 1
                }
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn copied_bytes_refuse_public_admission_at_one_below_the_malformed_exact_fit() {
    assert_refusal(
        ReplicaDecodeLimits::new(7, 91, 2780).unwrap(),
        ReplicaBudgetError::CopiedBytes,
    );
}

#[test]
fn entries_refuse_public_admission_before_sticky_root_override() {
    assert_refusal(
        ReplicaDecodeLimits::new(7, 90, 2781).unwrap(),
        ReplicaBudgetError::Entries,
    );
}

#[test]
fn depth_refuses_public_admission_before_sticky_root_override() {
    assert_refusal(
        ReplicaDecodeLimits::new(6, 91, 2781).unwrap(),
        ReplicaBudgetError::Depth,
    );
}

fn assert_refusal(limits: ReplicaDecodeLimits, category: ReplicaBudgetError) {
    let world = support::fixture();
    let inputs = malformed(&world);
    let legacy = admit(&world, inputs.to_vec(), authority()).unwrap();
    assert_eq!(
        evaluate(&legacy, inputs[0].job),
        Err(Rejection::InvalidProposal)
    );
    let refused = limits.with_scope::<_, (), _>(|scope| {
        let direct = admit_with_scope(&world, &inputs, authority(), scope);
        assert_eq!(direct, Err(category));
        direct.map_err(ScopedError::Budget)
    });
    assert_eq!(refused, Err(ScopedError::Budget(category)));
}
