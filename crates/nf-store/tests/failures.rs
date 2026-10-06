mod support;
use nf_store::{Boundary, KnownFrontiers, RequestStatus, Store, StoreError};
#[test]
fn hook_failure_before_commit_rolls_back_but_after_commit_requires_recovery() {
    for boundary in [Boundary::BeforeCommit, Boundary::AfterCommit] {
        let scratch = support::Scratch::new();
        let world = support::strategic_world();
        let intent = support::intent(&world, 7, -25);
        let frontier = support::frontier(&world, &intent);
        let mut store = Store::create(scratch.db(), &world).unwrap();
        store
            .prepare(&frontier, &support::devices(&intent), &[])
            .unwrap();
        let (_, batch) = support::batch(&world, &frontier);
        let result = store.commit_with_hook(&batch, &mut |point| {
            if point == boundary {
                Err(StoreError::Io)
            } else {
                Ok(())
            }
        });
        assert!(matches!(
            result,
            Err(StoreError::Io | StoreError::UncertainCommit)
        ));
        if boundary == Boundary::AfterCommit {
            assert_eq!(
                store.query(&intent, support::device()),
                Err(StoreError::Quarantined)
            );
        }
        drop(store);
        let spec = world.to_spec();
        let store = Store::open_existing(
            scratch.db(),
            KnownFrontiers::genesis(spec.universe, spec.history),
        )
        .unwrap();
        let status = store.query(&intent, support::device()).unwrap().unwrap();
        assert_eq!(
            matches!(status, RequestStatus::Committed { .. }),
            boundary == Boundary::AfterCommit
        );
        assert_eq!(
            store.outbox().count(),
            usize::from(boundary == Boundary::AfterCommit)
        );
    }
}
#[test]
fn real_sqlite_full_rolls_back_without_success_and_retains_pending_binding() {
    let scratch = support::Scratch::new();
    let mut store = Store::create(scratch.db(), &support::strategic_world()).unwrap();
    let pages = store.page_count().unwrap();
    store.lower_page_limit(pages).unwrap();
    let mut failed = false;
    for n in 1..=80 {
        let intent = support::intent(store.world(), n, 1);
        let frontier = support::frontier(store.world(), &intent);
        let result = store.prepare(&frontier, &support::devices(&intent), &[]);
        if result == Err(StoreError::Full) {
            failed = true;
            assert_eq!(store.query(&intent, support::device()).unwrap(), None);
            break;
        }
        result.unwrap();
        let (_, batch) = support::batch(store.world(), &frontier);
        let result = store.commit(&batch);
        if result == Err(StoreError::Full) {
            failed = true;
            let spec = store.world().to_spec();
            drop(store);
            let store = Store::open_existing(
                scratch.db(),
                KnownFrontiers::genesis(spec.universe, spec.history),
            )
            .unwrap();
            assert!(matches!(
                store.query(&intent, support::device()).unwrap(),
                Some(RequestStatus::Pending { .. })
            ));
            assert!(
                !store
                    .outbox()
                    .any(|entry| entry.operation == intent.operation)
            );
            break;
        }
        result.unwrap();
        store.mark_outbox_delivered(intent.operation).unwrap();
    }
    assert!(
        failed,
        "owned SQLite page quota must force actual SQLITE_FULL"
    );
}
#[test]
fn full_database_during_settlement_never_acknowledges_pending_request() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let intent = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &intent);
    let mut store = Store::create(scratch.db(), &world).unwrap();
    store
        .prepare(&frontier, &support::devices(&intent), &[])
        .unwrap();
    store.lower_page_limit(store.page_count().unwrap()).unwrap();
    let (_, batch) = support::batch(&world, &frontier);
    assert_eq!(store.commit(&batch), Err(StoreError::Full));
    drop(store);
    let spec = world.to_spec();
    let store = Store::open_existing(
        scratch.db(),
        KnownFrontiers::genesis(spec.universe, spec.history),
    )
    .unwrap();
    assert!(matches!(
        store.query(&intent, support::device()).unwrap(),
        Some(RequestStatus::Pending { .. })
    ));
    assert_eq!(store.world(), &world);
    assert_eq!(store.outbox().count(), 0);
}
