use nf_contract::canonical::replica_budget::{ReplicaDecodeLimits, ReplicaUsage};
use nf_kernel::replica::admit_with_scope;
use nf_kernel::{MAX_ENTITIES, MAX_FRONTIER, MAX_OUTCOMES, MAX_PROVIDERS, Rejection, admit};
#[path = "replica_admission_combined/support.rs"]
mod combined_support;
mod support;

#[test]
fn actual_supported_world_and_intents_hit_the_combined_local_cap_before_any_charge() {
    let world = combined_support::retained_outcomes();
    let spec = world.to_spec();
    assert_eq!(
        spec.factions.len() + spec.relations.len() + spec.markets.len(),
        MAX_ENTITIES
    );
    assert_eq!(spec.providers.len(), MAX_PROVIDERS);
    assert_eq!(spec.registry.len(), MAX_PROVIDERS);
    assert_eq!(spec.revisions.len(), MAX_ENTITIES + MAX_PROVIDERS);
    assert_eq!(world.outcomes().len(), MAX_OUTCOMES);
    let inputs = combined_support::full_expected_intents(&world);
    assert_eq!(inputs.len(), MAX_FRONTIER);
    assert!(
        inputs
            .iter()
            .all(|intent| intent.expected == spec.revisions)
    );
    let snapshot_entries = combined_support::snapshot_entries(&world);
    let frontier_entries = inputs.len()
        + inputs
            .iter()
            .map(|value| value.expected.len())
            .sum::<usize>();
    assert_eq!(snapshot_entries, 6819);
    assert_eq!(frontier_entries, 10304);
    assert_eq!(snapshot_entries.checked_add(frontier_entries), Some(17123));
    assert_eq!(
        admit(&world, inputs.clone(), combined_support::authority()),
        Err(Rejection::Limit)
    );
    ReplicaDecodeLimits::new(1, 0, 0)
        .unwrap()
        .with_scope::<_, (), _>(|scope| {
            let direct = admit_with_scope(&world, &inputs, combined_support::authority(), scope);
            // Check the public ordinary result before the root can override sticky failures.
            assert_eq!(direct, Ok(Err(Rejection::Limit)));
            assert_eq!(
                scope.usage(),
                ReplicaUsage {
                    entries: 0,
                    copied_bytes: 0,
                    depth: 1
                }
            );
            Ok(())
        })
        .unwrap();
}
