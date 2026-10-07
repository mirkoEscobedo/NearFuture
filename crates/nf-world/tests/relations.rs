mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn relationship_is_symmetric_bounded_and_controlled_by_the_initiators_account() {
    let s = support::state();
    let f = s.factions().iter().find(|v| v.ordinal == 0).unwrap();
    let other = s.factions().iter().find(|v| v.ordinal == 1).unwrap().id;
    let mut c = support::colony(&s, 1, 7);
    c.action = WorldAction::SetRelationship {
        faction: f.id,
        other,
        score: -10000,
    };
    let p = evaluate(&s, WorldTick(1), &c).expect("supported NF relationship");
    assert_eq!(
        (p.reservation().credits(), p.reservation().supplies()),
        (0, 0)
    );
    let next = apply_plan(&s, p.committing_tick(), &p).unwrap();
    let pair = next
        .relations()
        .iter()
        .find(|r| r.left == f.id.min(other) && r.right == f.id.max(other))
        .unwrap();
    assert_eq!(pair.score, -10000);
    assert_eq!(next.relations().iter().filter(|r| r.score != 0).count(), 1);
    c.action = WorldAction::SetRelationship {
        faction: f.id,
        other,
        score: 10001,
    };
    assert_eq!(
        evaluate(&s, WorldTick(1), &c),
        Err(WorldError::InvalidValue)
    );
    c.action = WorldAction::SetRelationship {
        faction: f.id,
        other,
        score: 0,
    };
    c.actor = AccountId::from_bytes([99; 16]);
    assert_eq!(
        evaluate(&s, WorldTick(1), &c),
        Err(WorldError::Unauthorized)
    );
}
