mod support;
use nf_contract::identity::*;
use nf_store::{RequestStatus, Store, StoreError};
#[test]
fn bound_query_requires_retained_principal_device_and_scope_after_restart() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let intent = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &intent);
    let mut store = Store::create(scratch.db(), &world).unwrap();
    store
        .prepare(&frontier, &support::devices(&intent), &[])
        .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let store = Store::open_existing(scratch.db(), known).unwrap();
    let found = store
        .query_bound(intent.request, intent.actor, support::device(), known.scope)
        .unwrap()
        .unwrap();
    assert_eq!(
        found.status,
        RequestStatus::Pending {
            operation: intent.operation
        }
    );
    assert_ne!(found.binding_digest, [0; 32]);
    assert_eq!(
        store.query_bound(
            intent.request,
            AccountId::from_bytes([99; 16]),
            support::device(),
            known.scope
        ),
        Err(StoreError::RequestConflict)
    );
    assert_eq!(
        store.query_bound(
            intent.request,
            intent.actor,
            DeviceId::from_bytes([99; 16]),
            known.scope
        ),
        Err(StoreError::RequestConflict)
    );
    let mut scope = known.scope;
    scope.history = HistoryId::from_bytes([99; 16]);
    assert_eq!(
        store.query_bound(intent.request, intent.actor, support::device(), scope),
        Err(StoreError::Scope)
    );
    assert_eq!(
        store
            .query_bound(
                RequestId::from_bytes([99; 16]),
                intent.actor,
                support::device(),
                known.scope
            )
            .unwrap(),
        None
    );
}
