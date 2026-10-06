mod support;
use nf_store::{KnownFrontiers, Store};
#[test]
fn explicit_initialization_and_existing_history_recovery_preserve_world() {
    let scratch = support::Scratch::new();
    let world = support::world();
    let scope = world.to_spec();
    let store = Store::create(scratch.db(), &world).unwrap();
    assert_eq!(store.world(), &world);
    drop(store);
    let restored = Store::open_existing(
        scratch.db(),
        KnownFrontiers::genesis(scope.universe, scope.history),
    )
    .unwrap();
    assert_eq!(restored.world(), &world);
    assert!(Store::create(scratch.db(), &world).is_err());
    assert!(
        Store::open_existing(
            scratch.0.join("absent.sqlite"),
            KnownFrontiers::genesis(scope.universe, scope.history)
        )
        .is_err()
    );
}
