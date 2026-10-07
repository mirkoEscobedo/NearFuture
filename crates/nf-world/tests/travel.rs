mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn abstract_travel_reserves_supplies_locks_fleet_and_arrives_after_three_committed_ticks() {
    let s = support::state();
    let f = s.factions().iter().find(|f| f.ordinal == 0).unwrap();
    let fleet = s.fleets().iter().find(|v| v.faction == f.id).unwrap();
    let destination = s.systems().iter().find(|v| v.ordinal == 1).unwrap().id;
    let mut c = support::colony(&s, 1, 7);
    c.action = WorldAction::Travel {
        fleet: fleet.id,
        destination,
    };
    let p = evaluate(&s, WorldTick(10), &c).expect("legal abstract travel");
    assert_eq!(
        (p.reservation().credits(), p.reservation().supplies()),
        (0, 20)
    );
    let departed = apply_plan(&s, p.committing_tick(), &p).unwrap();
    assert_eq!(departed.schedules()[0].due, WorldTick(13));
    assert!(matches!(
        departed
            .fleets()
            .iter()
            .find(|v| v.id == fleet.id)
            .unwrap()
            .location,
        FleetLocation::InTransit(_)
    ));
    c.expected_revision = departed.revision();
    assert_eq!(
        evaluate(&departed, WorldTick(11), &c),
        Err(WorldError::Locked)
    );
    assert!(due_events(&departed, WorldTick(12)).unwrap().is_empty());
    let events = due_events(&departed, WorldTick(13)).unwrap();
    let arrived = apply_due(&departed, WorldTick(13), &events).unwrap();
    assert_eq!(
        arrived
            .fleets()
            .iter()
            .find(|v| v.id == fleet.id)
            .unwrap()
            .location,
        FleetLocation::Docked(destination)
    );
    assert!(arrived.schedules().is_empty());
}
