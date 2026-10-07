mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn decoded_industry_cannot_appear_without_its_canonical_resource_cost() {
    let s = support::state();
    let m = s.markets().iter().find(|m| m.ordinal == 0).unwrap();
    let mut b = encode_component(&s, WorldTick(0)).unwrap();
    let pos = b.windows(16).position(|w| w == m.id.as_bytes()).unwrap();
    b[pos + 50] = 2;
    assert_eq!(
        decode_component(&b, WorldTick(0)),
        Err(WorldError::Resources)
    );
}
#[test]
fn expired_unprocessed_timer_blocks_new_action_instead_of_silently_skipping_it() {
    let s = support::state();
    let market = s.markets().iter().find(|m| m.ordinal == 0).unwrap().id;
    let mut c = support::colony(&s, 1, 7);
    c.action = WorldAction::BuildIndustry {
        market,
        kind: IndustryKind::Farming,
    };
    let started = apply_plan(&s, WorldTick(1), &evaluate(&s, WorldTick(1), &c).unwrap()).unwrap();
    let next = support::colony(&started, 1, 8);
    assert_eq!(
        evaluate(&started, WorldTick(5), &next),
        Err(WorldError::InvalidValue)
    );
}
