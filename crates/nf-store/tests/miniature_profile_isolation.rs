mod support;
use nf_contract::identity::AuthorityTerm;
use nf_store::{KnownFrontiers, Store, StoreError, miniature::*};
#[test]
fn miniature_reader_refuses_profile1_without_rewrite_or_migration() {
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let store = Store::create(scratch.db(), &world).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let before = std::fs::read(scratch.db()).unwrap();
    let result = MiniatureStore::open_existing(
        scratch.db(),
        MiniatureKnownFrontiers {
            storage: known,
            minimum_authority_term: AuthorityTerm(0),
        },
        AuthConfig::default(),
    );
    assert!(matches!(
        result,
        Err(MiniatureStoreError::Storage(StoreError::UnsupportedSchema))
    ));
    assert_eq!(std::fs::read(scratch.db()).unwrap(), before);
    let spec = world.to_spec();
    assert!(
        Store::open_existing(
            scratch.db(),
            KnownFrontiers::genesis(spec.universe, spec.history)
        )
        .is_ok()
    );
}
