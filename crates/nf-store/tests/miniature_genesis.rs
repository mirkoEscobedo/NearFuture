mod miniature_support;
use miniature_support::*;
use nf_identity::model::MembershipRepository;
use nf_store::miniature::MiniatureStore;
#[test]
fn pinned_genesis_creates_profile2_without_preclaim_or_membership_changes() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let bytes = f.bytes();
    let mut store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &bytes,
        f.policy.clone(),
        &mut f.signer,
    )
    .expect("a real owner signature must initialize pinned profile2");
    assert_eq!(store.world().metadata().tick.0, 0);
    assert_eq!(
        store.world().component(),
        &nf_world::generate(f.spec.genesis).unwrap()
    );
    assert_eq!(
        store.load_membership(f.policy.scope).unwrap(),
        Some(f.membership)
    );
    assert!(store.authority().is_none());
    let known = store.known_frontiers().unwrap();
    drop(store);
    let sql = rusqlite::Connection::open(scratch.db()).unwrap();
    assert_eq!(
        sql.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
            .unwrap(),
        2
    );
    drop(sql);
    let reopened = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth)
        .expect("exact profile2 genesis must reopen");
    assert_eq!(reopened.world().metadata().event_sequence.0, 0);
}
