mod miniature_support;
#[test]
fn miniature_world_validates_typed_component_without_creating_legacy_mirrors() {
    let world = miniature_support::world();
    assert_eq!(world.component().systems().len(), 3);
    assert_eq!(world.component().markets().len(), 6);
    assert_eq!(world.metadata().tick.0, 0);
    assert!(world.outcomes().is_empty());
}
