mod support;
use nf_store::{KnownFrontiers, Store, StoreError};
fn known(world: &nf_kernel::World) -> KnownFrontiers {
    let spec = world.to_spec();
    KnownFrontiers::genesis(spec.universe, spec.history)
}
#[test]
fn corrupt_unknown_and_tampered_histories_fail_closed_without_initialization() {
    for case in 0..4 {
        let scratch = support::Scratch::new();
        let world = support::strategic_world();
        drop(Store::create(scratch.db(), &world).unwrap());
        if case == 0 {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(scratch.db())
                .unwrap();
            file.set_len(50).unwrap();
        } else {
            let connection = rusqlite::Connection::open(scratch.db()).unwrap();
            let sql = match case {
                1 => "PRAGMA user_version=2",
                2 => "UPDATE history_meta SET digest=zeroblob(32)",
                _ => {
                    "UPDATE aggregate_state SET revision=zeroblob(8); UPDATE module_state SET draws=x'0000000000000001'"
                }
            };
            connection.execute_batch(sql).unwrap();
        }
        let before = std::fs::read(scratch.db()).unwrap();
        assert!(matches!(
            Store::open_existing(scratch.db(), known(&world)),
            Err(StoreError::Corrupt | StoreError::UnsupportedSchema)
        ));
        assert_eq!(std::fs::read(scratch.db()).unwrap(), before);
    }
}
#[test]
fn consistent_backup_requires_all_known_world_and_store_frontiers() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let mut store = Store::create(scratch.db(), &world).unwrap();
    let backup = scratch.0.join("old.sqlite");
    let old = store.backup_to(&backup).unwrap();
    let intent = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &intent);
    store
        .prepare(&frontier, &support::devices(&intent), &[])
        .unwrap();
    let accepted = store.known_frontiers().unwrap();
    assert!(matches!(
        Store::open_existing(&backup, accepted),
        Err(StoreError::StaleBackup)
    ));
    let (_, batch) = support::batch(&world, &frontier);
    store.commit(&batch).unwrap();
    let committed = store.known_frontiers().unwrap();
    assert!(matches!(
        Store::open_existing(&backup, committed),
        Err(StoreError::StaleBackup)
    ));
    assert_eq!(Store::open_existing(&backup, old).unwrap().world(), &world);
}
#[test]
fn unversioned_trigger_view_and_user_index_are_refused_before_any_write() {
    for sql in [
        "CREATE TRIGGER unexpected AFTER INSERT ON journal BEGIN DELETE FROM membership; END",
        "CREATE VIEW unexpected AS SELECT * FROM membership",
        "CREATE INDEX unexpected ON journal(kind)",
    ] {
        let scratch = support::Scratch::new();
        let world = support::strategic_world();
        drop(Store::create(scratch.db(), &world).unwrap());
        let connection = rusqlite::Connection::open(scratch.db()).unwrap();
        connection.execute_batch(sql).unwrap();
        drop(connection);
        let before = std::fs::read(scratch.db()).unwrap();
        assert!(matches!(
            Store::open_existing(scratch.db(), known(&world)),
            Err(StoreError::UnsupportedSchema)
        ));
        assert_eq!(std::fs::read(scratch.db()).unwrap(), before);
    }
}
