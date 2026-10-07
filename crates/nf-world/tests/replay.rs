mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn typed_checkpoint_replay_reproduces_same_tick_ordered_arrival_and_construction() {
    let s = support::state();
    let faction = s.factions().iter().find(|f| f.ordinal == 0).unwrap().id;
    let fleet = s.fleets().iter().find(|f| f.faction == faction).unwrap().id;
    let destination = s.systems().iter().find(|s| s.ordinal == 1).unwrap().id;
    let mut travel = support::colony(&s, 1, 7);
    travel.action = WorldAction::Travel { fleet, destination };
    let departed = apply_plan(
        &s,
        WorldTick(1),
        &evaluate(&s, WorldTick(1), &travel).unwrap(),
    )
    .unwrap();
    let restored = decode_component(
        &encode_component(&departed, WorldTick(1)).unwrap(),
        WorldTick(1),
    )
    .unwrap();
    assert_eq!(restored, departed);
    let market = restored
        .markets()
        .iter()
        .find(|m| m.ordinal == 2)
        .unwrap()
        .id;
    let mut build = support::colony(&restored, 3, 8);
    build.action = WorldAction::BuildIndustry {
        market,
        kind: IndustryKind::Workshop,
    };
    let pending = apply_plan(
        &restored,
        WorldTick(2),
        &evaluate(&restored, WorldTick(2), &build).unwrap(),
    )
    .unwrap();
    let bytes = encode_component(&pending, WorldTick(2)).unwrap();
    let checkpoint = decode_component(&bytes, WorldTick(2)).unwrap();
    assert!(decode_component(&bytes, WorldTick(4)).is_err());
    assert!(due_events(&checkpoint, WorldTick(3)).unwrap().is_empty());
    let events = due_events(&checkpoint, WorldTick(4)).unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.windows(2).all(|w| w[0].id < w[1].id));
    let mut reversed = events.clone();
    reversed.reverse();
    assert!(apply_due(&checkpoint, WorldTick(4), &reversed).is_err());
    assert!(apply_due(&checkpoint, WorldTick(4), &events[..1]).is_err());
    let replayed = apply_due(&checkpoint, WorldTick(4), &events).unwrap();
    let direct = apply_due(&pending, WorldTick(4), &events).unwrap();
    assert_eq!(
        encode_component(&replayed, WorldTick(4)).unwrap(),
        encode_component(&direct, WorldTick(4)).unwrap()
    );
    assert!(replayed.schedules().is_empty());
    assert_eq!(replayed.revision(), AggregateRevision(4));
    assert!(apply_due(&replayed, WorldTick(4), &events).is_err());
}
