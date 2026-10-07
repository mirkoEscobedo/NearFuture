use super::{snapshot_support, support};
use crate::World;
use crate::replica::snapshot as scoped_snapshot;
use nf_contract::canonical::replica_budget::{ReplicaDecodeLimits, ReplicaUsage};

#[test]
fn scoped_snapshot_genesis_bytes_hash_and_exact_logical_usage_match() {
    let world = support::fixture();
    let expected = crate::encode_snapshot(&world).unwrap();
    assert_eq!(expected.len(), 1153);
    let (actual, usage) = ReplicaDecodeLimits::new(6, 44, 1249)
        .unwrap()
        .with_scope(|scope| Ok((scoped_snapshot::encode(&world, scope)?, scope.usage())))
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        usage,
        ReplicaUsage {
            entries: 44,
            copied_bytes: 1249,
            depth: 1
        }
    );
    let actual = ReplicaDecodeLimits::new(6, 44, 1249)
        .unwrap()
        .with_scope(|scope| scoped_snapshot::hash(&world, scope))
        .unwrap();
    assert_eq!(actual, crate::state_hash(&world).unwrap());
}

#[test]
fn scoped_snapshot_parity_covers_reordered_valid_vectors_and_varied_manifests() {
    assert_parity(&snapshot_support::varied());
}

#[test]
fn scoped_snapshot_parity_covers_larger_valid_collections() {
    let world = snapshot_support::larger();
    assert!(world.to_spec().factions.len() > 100);
    assert_parity(&world);
}

#[test]
fn scoped_snapshot_parity_covers_actual_successful_and_rejected_settlement_outcomes() {
    assert_parity(&snapshot_support::settled(false));
    assert_parity(&snapshot_support::settled(true));
}

fn assert_parity(world: &World) {
    let expected = crate::encode_snapshot(world).unwrap();
    let actual = ReplicaDecodeLimits::new(6, 16384, 1_048_576)
        .unwrap()
        .with_scope(|scope| scoped_snapshot::encode(world, scope))
        .unwrap();
    assert_eq!(actual, expected);
    let actual = ReplicaDecodeLimits::new(6, 16384, 1_048_576)
        .unwrap()
        .with_scope(|scope| scoped_snapshot::hash(world, scope))
        .unwrap();
    assert_eq!(actual, crate::state_hash(world).unwrap());
}
