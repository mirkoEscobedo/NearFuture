mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn relationship_frontier_orders_the_shared_pair_entity_instead_of_initiating_faction() {
    let s = support::state();
    let a = s.factions().iter().find(|f| f.ordinal == 0).unwrap().id;
    let b = s.factions().iter().find(|f| f.ordinal == 1).unwrap().id;
    let c = s.factions().iter().find(|f| f.ordinal == 2).unwrap().id;
    let mut ac = support::colony(&s, 1, 8);
    ac.action = WorldAction::SetRelationship {
        faction: a,
        other: c,
        score: 100,
    };
    let mut bc = support::colony(&s, 3, 9);
    bc.action = WorldAction::SetRelationship {
        faction: b,
        other: c,
        score: 200,
    };
    let p = plan_reservations(&s, WorldTick(1), &[bc, ac.clone()]).unwrap();
    assert_eq!(p.winner().unwrap().candidate().operation, ac.operation);
}
