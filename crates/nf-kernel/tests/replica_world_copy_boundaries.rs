use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ReplicaUsage, ScopedError,
};
use nf_contract::identity::{AuthorityTerm, RuntimeSession, WorldTick};
use nf_kernel::replica::clone_world_with_scope;
use nf_kernel::{AuthorityContext, MissingPolicy, Settlement, World, admit, settle};
mod support;

#[test]
fn exact_quota_copies_genesis_and_real_settled_outcomes() {
    let genesis = support::fixture();
    let settled = settle_two_real_jobs(&genesis);
    // Independently approved literals: genesis1068/32; two actual outcomes add72/2.
    for (world, entries, copied) in [(&genesis, 32, 1068), (&settled, 34, 1140)] {
        let (copy, usage) = ReplicaDecodeLimits::new(6, entries, copied)
            .unwrap()
            .with_scope::<_, (), _>(|scope| {
                let copy = clone_world_with_scope(world, scope).map_err(ScopedError::Budget)?;
                assert_eq!(&copy, world);
                Ok((copy, scope.usage()))
            })
            .unwrap();
        assert_eq!(&copy, world);
        assert_eq!(
            usage,
            ReplicaUsage {
                entries: u64::from(entries),
                copied_bytes: copied,
                depth: 1,
            }
        );
    }
}

#[test]
fn entries_refuse_public_copies_before_sticky_root_override() {
    let genesis = support::fixture();
    let settled = settle_two_real_jobs(&genesis);
    for (world, limits) in [
        (&genesis, ReplicaDecodeLimits::new(6, 31, 1068).unwrap()),
        (&settled, ReplicaDecodeLimits::new(6, 33, 1140).unwrap()),
    ] {
        assert_public_refusal(world, limits, ReplicaBudgetError::Entries);
    }
}

#[test]
fn depth_refuses_public_copies_before_sticky_root_override() {
    let genesis = support::fixture();
    let settled = settle_two_real_jobs(&genesis);
    // World registry -> manifest -> nested set -> element reaches depth6.
    for (world, limits) in [
        (&genesis, ReplicaDecodeLimits::new(5, 32, 1068).unwrap()),
        (&settled, ReplicaDecodeLimits::new(5, 34, 1140).unwrap()),
    ] {
        assert_public_refusal(world, limits, ReplicaBudgetError::Depth);
    }
}

fn assert_public_refusal(world: &World, limits: ReplicaDecodeLimits, category: ReplicaBudgetError) {
    let refused = limits.with_scope::<_, (), _>(|scope| {
        let direct = clone_world_with_scope(world, scope);
        // Inspect the public helper before root completion can mask a wrong return.
        assert_eq!(direct, Err(category));
        direct.map_err(ScopedError::Budget)
    });
    assert_eq!(refused, Err(ScopedError::Budget(category)));
}

fn settle_two_real_jobs(genesis: &World) -> World {
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(
        genesis,
        vec![
            support::peace_intent(genesis, 1),
            support::market_intent(genesis, 2, 7),
        ],
        authority,
    )
    .unwrap();
    let Settlement::Committed { world, batch } = settle(
        genesis,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("the two real static jobs must recompute and commit")
    };
    assert_eq!(batch.outcomes.len(), 2);
    assert_eq!(world.outcomes(), batch.outcomes.as_slice());
    assert!(
        world
            .outcomes()
            .iter()
            .all(|value| value.rejection.is_none())
    );
    assert_eq!(world.view().tick(), WorldTick(1));
    assert!(genesis.outcomes().is_empty());
    *world
}
