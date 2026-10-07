mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn identical_component_after_an_empty_tick_cannot_apply_an_old_tick_plan() {
    let s = support::state();
    let c = support::colony(&s, 1, 7);
    let p = evaluate(&s, WorldTick(1), &c).unwrap();
    assert_eq!(
        apply_plan(&s, WorldTick(2), &p).err(),
        Some(WorldError::StaleRevision)
    );
}
