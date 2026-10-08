use crate::{Error, Scratch, TradeFixture, TradeStore};
use nf_store::{
    supplies::{ChallengeRequest, KnownSuppliesFrontiers, SuppliesStore, SuppliesStoreError},
    trade::KnownTradeFrontiers,
};
fn snapshot(scratch: &Scratch) -> (Vec<u8>, Vec<std::ffi::OsString>) {
    let mut names: Vec<_> = std::fs::read_dir(scratch.db().parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    (std::fs::read(scratch.db()).unwrap(), names)
}
#[test]
fn genuine_profile4_reopens_and_refuses_trade_selection_without_migration() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &f.supplies.policy, &f.supplies.membership).unwrap();
    let issue = f.supplies.issuance();
    let proof = f
        .supplies
        .attempt(&mut store, ChallengeRequest::Issue(&issue));
    assert_eq!(store.issue(&issue, proof).unwrap().unwrap().revision, 1);
    let known = store.known_frontiers().unwrap();
    drop(store);
    let before = snapshot(&scratch);
    let trade_known = KnownTradeFrontiers {
        scope: known.scope,
        revision: known.revision,
        membership_revision: known.membership_revision,
        clock_tick: 0,
        clock_revision: 0,
    };
    assert!(matches!(
        TradeStore::open_existing(scratch.db(), &f.policy, trade_known),
        Err(Error::Supplies(SuppliesStoreError::UnsupportedProfile))
    ));
    assert_eq!(
        snapshot(&scratch),
        before,
        "selection must not upgrade a genuine4 database"
    );
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.supplies.policy, known).unwrap();
    let query = f.supplies.query();
    let proof = f
        .supplies
        .attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(store.balance(&query, proof).unwrap().available, 25);
    assert_eq!(store.known_frontiers().unwrap(), known);
}
#[test]
fn genuine_profile6_reopens_and_refuses_supplies_selection_without_migration() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let store = f.funded(&scratch);
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 2);
    drop(store);
    let before = snapshot(&scratch);
    let supplies_known = KnownSuppliesFrontiers {
        scope: known.scope,
        revision: known.revision,
        membership_revision: known.membership_revision,
    };
    assert!(matches!(
        SuppliesStore::open_existing(scratch.db(), &f.supplies.policy, supplies_known),
        Err(SuppliesStoreError::UnsupportedProfile)
    ));
    assert_eq!(
        snapshot(&scratch),
        before,
        "selection must not rewrite a genuine6 database"
    );
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, known).unwrap();
    assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [4, 0, 0, 0, 4, 0]);
    assert_eq!(store.known_frontiers().unwrap(), known);
}

#[test]
fn current_profile6_shape_with_marker5_is_refused_without_migration() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let store = f.funded(&scratch);
    let known = store.known_frontiers().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(scratch.db()).unwrap();
    let version: i32 = connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 6);
    connection.pragma_update(None, "user_version", 5).unwrap();
    drop(connection);
    let before = snapshot(&scratch);
    // Marker control only: this is NOT a genuine historical profile5 database or migration proof.
    assert!(matches!(
        TradeStore::open_existing(scratch.db(), &f.policy, known),
        Err(Error::Supplies(SuppliesStoreError::UnsupportedProfile))
    ));
    assert_eq!(
        snapshot(&scratch),
        before,
        "refusal must not rewrite the marker5 database"
    );
}
