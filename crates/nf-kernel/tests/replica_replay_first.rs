use nf_contract::canonical::replica_budget::{
    ReplicaBudgetError, ReplicaDecodeLimits, ReplicaUsage, ScopedError,
};
use nf_contract::identity::{AuthorityTerm, RuntimeSession};
use nf_kernel::replica::apply_batch_with_scope;
use nf_kernel::*;
mod support;

// SOURCE proposal: API does not exist yet. Not formatted, compiled, run or applied.
// Arithmetic is hand-derived pending Root acceptance, not a runtime observation.
#[test]
fn committed_market_batch_replay_keeps_one_fatal_cumulative_scope() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let frontier = admit(
        &world,
        vec![support::market_intent(&world, 1, 7)],
        authority,
    )
    .unwrap();
    let Settlement::Committed {
        world: expected,
        batch,
    } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap()
    else {
        panic!("one genuine committed static market job");
    };
    // Existing encoders and settlement are fixture preparation outside the replay scope.
    let retained_world = encode_snapshot(&world).unwrap();
    let retained_batch = encode_batch(&batch).unwrap();
    let persisted = decode_batch(&retained_batch).unwrap();
    let refusal = ReplicaDecodeLimits::new(8, 502, 15181)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let first = apply_batch_with_scope(&world, &persisted, scope)
                .map_err(ScopedError::Budget)?
                .unwrap();
            assert_eq!(first, *expected);
            assert_eq!(
                scope.usage(),
                ReplicaUsage {
                    entries: 251,
                    copied_bytes: 7591,
                    depth: 1,
                }
            );
            // One byte short of two full replays. No scope reset or independent root.
            let second = apply_batch_with_scope(&world, &persisted, scope);
            assert_eq!(second, Err(ReplicaBudgetError::CopiedBytes));
            assert_eq!(scope.failure(), Some(ReplicaBudgetError::CopiedBytes));
            second.map_err(ScopedError::Budget)
        });
    assert_eq!(
        refusal,
        Err(ScopedError::Budget(ReplicaBudgetError::CopiedBytes))
    );
    assert_eq!(encode_snapshot(&world).unwrap(), retained_world);
    assert_eq!(encode_batch(&persisted).unwrap(), retained_batch);
}
