mod driver_support;
use nf_contract::identity::*;
use nf_world_driver::Driver;
#[test]
fn explicit_pinned_genesis_uses_existing_real_vault_and_single_sqlite_store_without_preclaim() {
    let fixture = driver_support::Fixture::new();
    let options = fixture.options.clone();
    let driver = Driver::create(options.clone()).expect("real vault signed pinned genesis");
    let first = driver.status().unwrap();
    assert_eq!(
        first.world.component(),
        &nf_world::generate(options.genesis.genesis).unwrap()
    );
    assert_eq!(first.world.metadata().tick, WorldTick(0));
    assert_eq!(first.world.metadata().event_sequence, EventSeq(0));
    assert_eq!(first.known.storage.membership_revision, Some(2));
    assert_eq!(first.known.storage.store_revision, 0);
    assert!(first.authority.is_none());
    assert!(first.pending.is_none());
    assert_eq!(driver.status().unwrap(), first);
    assert!(options.database.is_file());
    assert_eq!(
        std::fs::read_dir(options.game_save_root).unwrap().count(),
        0
    );
}
