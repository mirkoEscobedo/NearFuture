mod support;
use nf_store::{KnownFrontiers, RequestStatus, Reservation, Store, StoreError};
#[test]
fn durable_pending_frontier_and_reservation_restart_then_atomic_commit() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let intent = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &intent);
    let reservation = Reservation {
        operation: intent.operation,
        market: nf_contract::identity::EntityId::from_bytes([30; 16]),
        amount: 25,
    };
    let mut store = Store::create(scratch.db(), &world).unwrap();
    assert_eq!(store.outbox().count(), 0);
    store
        .prepare(&frontier, &support::devices(&intent), &[reservation])
        .unwrap();
    assert_eq!(store.outbox().count(), 0);
    assert_eq!(
        store.query(&intent, support::device()).unwrap(),
        Some(RequestStatus::Pending {
            operation: intent.operation
        })
    );
    let known = store.known_frontiers().unwrap();
    let snapshot = store.snapshot().unwrap();
    drop(store);
    let mut store = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(store.snapshot().unwrap().digest(), snapshot.digest());
    assert_eq!(
        nf_kernel::encode_frontier(store.pending().unwrap()).unwrap(),
        nf_kernel::encode_frontier(&frontier).unwrap()
    );
    assert_eq!(
        store.reservations().copied().collect::<Vec<_>>(),
        vec![reservation]
    );
    let (restored, batch) = support::batch(store.world(), store.pending().unwrap());
    let ack = store.commit(&batch).unwrap();
    assert_eq!(ack.sequence(), restored.view().event_seq());
    assert_eq!(ack.state_hash(), nf_kernel::state_hash(&restored).unwrap());
    assert_eq!(store.world(), &restored);
    assert_eq!(store.reservations().count(), 0);
    assert_eq!(store.outbox().count(), 1);
    let known = store.known_frontiers().unwrap();
    drop(store);
    let store = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(store.world(), &restored);
    assert_eq!(store.outbox().count(), 1);
}
#[test]
fn binding_conflicts_survive_restart_and_compaction_while_same_id_queries_result() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let intent = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &intent);
    let mut store = Store::create(scratch.db(), &world).unwrap();
    store
        .prepare(&frontier, &support::devices(&intent), &[])
        .unwrap();
    let (_, batch) = support::batch(&world, &frontier);
    store.commit(&batch).unwrap();
    let status = store.query(&intent, support::device()).unwrap();
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(store.query(&intent, support::device()).unwrap(), status);
    let mut changed = intent.clone();
    changed.command = nf_kernel::Command::AdjustMarket {
        market: nf_contract::identity::EntityId::from_bytes([30; 16]),
        delta: -24,
    };
    assert_eq!(
        store.query(&changed, support::device()),
        Err(StoreError::RequestConflict)
    );
    assert_eq!(
        store.query(
            &intent,
            nf_contract::identity::DeviceId::from_bytes([92; 16])
        ),
        Err(StoreError::RequestConflict)
    );
    store.mark_outbox_delivered(intent.operation).unwrap();
    drop(store);
    let spec = world.to_spec();
    let store = Store::open_existing(
        scratch.db(),
        KnownFrontiers::genesis(spec.universe, spec.history),
    )
    .unwrap();
    assert_eq!(store.query(&intent, support::device()).unwrap(), status);
    assert_eq!(store.outbox().count(), 0);
}
#[test]
fn automatic_checkpoint_keeps_old_outcomes_and_replays_only_new_batches() {
    let scratch = support::Scratch::new();
    let mut store = Store::create(scratch.db(), &support::strategic_world()).unwrap();
    let first = support::intent(store.world(), 1, 1);
    for n in 1..=50 {
        let intent = support::intent(store.world(), n, 1);
        let frontier = support::frontier(store.world(), &intent);
        store
            .prepare(&frontier, &support::devices(&intent), &[])
            .unwrap();
        let (_, batch) = support::batch(store.world(), &frontier);
        store.commit(&batch).unwrap();
        store.mark_outbox_delivered(intent.operation).unwrap();
    }
    let expected = store.world().clone();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let store = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(store.world(), &expected);
    assert!(matches!(
        store.query(&first, support::device()).unwrap(),
        Some(RequestStatus::Committed { .. })
    ));
}
