use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ReplicaUsage, ScopedError,
};
use nf_kernel::replica::clone_world_with_scope;
mod support;

#[test]
fn a_second_owned_world_copy_is_refused_by_the_same_cumulative_scope() {
    let world = support::fixture();
    // Independent existing fixture vector: World C1068/E32, maximum depth6.
    let limits = ReplicaDecodeLimits::new(6, 64, 1068).unwrap();
    let result = limits.with_scope::<_, (), _>(|scope| {
        let first = clone_world_with_scope(&world, scope).map_err(ScopedError::Budget)?;
        assert_eq!(first, world);
        let first_usage = scope.usage();
        let second = clone_world_with_scope(&first, scope);
        // Quota bypass is the intended RED; inspect it before passive usage assertions.
        assert_eq!(second, Err(ReplicaBudgetError::CopiedBytes));
        assert_eq!(
            first_usage,
            ReplicaUsage {
                entries: 32,
                copied_bytes: 1068,
                depth: 1,
            }
        );
        second.map_err(ScopedError::Budget)
    });
    assert_eq!(
        result,
        Err(ScopedError::Budget(ReplicaBudgetError::CopiedBytes))
    );
}
