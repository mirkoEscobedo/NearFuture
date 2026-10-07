mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn exhausted_supplies_never_create_a_sixth_departure_or_negative_balance() {
    let mut s = support::state();
    let faction = s.factions().iter().find(|v| v.ordinal == 0).unwrap().id;
    let fleet = s.fleets().iter().find(|v| v.faction == faction).unwrap().id;
    let a = s.systems().iter().find(|v| v.ordinal == 0).unwrap().id;
    let b = s.systems().iter().find(|v| v.ordinal == 1).unwrap().id;
    for i in 0..5 {
        let tick = WorldTick(1 + i * 4);
        let mut c = support::colony(&s, 1, (i + 1) as u8);
        c.action = WorldAction::Travel {
            fleet,
            destination: if i % 2 == 0 { b } else { a },
        };
        let p = evaluate(&s, tick, &c).unwrap();
        s = apply_plan(&s, tick, &p).unwrap();
        let end = WorldTick(tick.0 + 3);
        s = apply_due(&s, end, &due_events(&s, end).unwrap()).unwrap();
        validate_at(&s, end).unwrap();
        assert_eq!(
            decode_component(&encode_component(&s, end).unwrap(), end).unwrap(),
            s
        );
    }
    let f = s.factions().iter().find(|v| v.id == faction).unwrap();
    assert_eq!((f.supplies, f.spent_supplies), (0, 100));
    let mut c = support::colony(&s, 1, 9);
    c.action = WorldAction::Travel {
        fleet,
        destination: a,
    };
    assert_eq!(
        evaluate(&s, WorldTick(21), &c).err(),
        Some(WorldError::Resources)
    );
    assert!(s.schedules().is_empty());
}
#[test]
fn frontier_count_duplicate_and_zero_tick_fail_before_any_reservation() {
    let s = support::state();
    let c = support::colony(&s, 1, 1);
    assert_eq!(
        plan_reservations(&s, WorldTick(1), &vec![c.clone(); 65]).err(),
        Some(WorldError::Limit)
    );
    assert_eq!(
        plan_reservations(&s, WorldTick(1), &[c.clone(), c]).err(),
        Some(WorldError::Duplicate)
    );
    assert_eq!(
        plan_reservations(&s, WorldTick(0), &[]).err(),
        Some(WorldError::InvalidValue)
    );
}
