mod support;
use nf_contract::identity::*;
use nf_world::*;

fn scheduled(state: &State, tick: WorldTick, arrival: bool) -> State {
    let mut c = support::colony(state, 1, 7);
    c.action = if arrival {
        let faction = state.factions().iter().find(|f| f.ordinal == 0).unwrap().id;
        WorldAction::Travel {
            fleet: state
                .fleets()
                .iter()
                .find(|f| f.faction == faction)
                .unwrap()
                .id,
            destination: state.systems().iter().find(|s| s.ordinal == 1).unwrap().id,
        }
    } else {
        WorldAction::BuildIndustry {
            market: state.markets().iter().find(|m| m.ordinal == 0).unwrap().id,
            kind: IndustryKind::Farming,
        }
    };
    apply_plan(state, tick, &evaluate(state, tick, &c).unwrap()).unwrap()
}
fn altered_due(state: &State, tick: WorldTick, due: u64) -> Vec<u8> {
    let schedule = &state.schedules()[0];
    let mut bytes = encode_component(state, tick).unwrap();
    let mut identity = schedule.id.as_bytes().to_vec();
    identity.extend_from_slice(schedule.operation.as_bytes());
    let start = bytes
        .windows(identity.len())
        .position(|w| w == identity)
        .unwrap()
        + 32;
    bytes[start..start + 8].copy_from_slice(&due.to_le_bytes());
    bytes
}
#[test]
fn committed_decoder_rejects_one_over_and_far_future_deadlines_for_each_kind() {
    for (arrival, duration) in [(false, 2), (true, 3)] {
        let state = scheduled(&support::state(), WorldTick(1), arrival);
        assert_eq!(state.schedules()[0].due, WorldTick(1 + duration));
        let legal = encode_component(&state, WorldTick(1)).unwrap();
        assert_eq!(decode_component(&legal, WorldTick(1)).unwrap(), state);
        for due in [1 + duration + 1, 100] {
            assert_eq!(
                decode_component(&altered_due(&state, WorldTick(1), due), WorldTick(1)),
                Err(WorldError::InvalidValue)
            );
        }
        assert_eq!(
            encode_component(&state, WorldTick(0)),
            Err(WorldError::InvalidValue)
        );
    }
}
#[test]
fn horizon_validation_near_maximum_tick_uses_remaining_distance_without_overflow() {
    for (arrival, duration) in [(false, 2), (true, 3)] {
        let tick = WorldTick(u64::MAX - duration);
        let state = scheduled(&support::state(), tick, arrival);
        assert_eq!(state.schedules()[0].due, WorldTick(u64::MAX));
        let bytes = encode_component(&state, tick).unwrap();
        assert_eq!(decode_component(&bytes, tick).unwrap(), state);
        assert_eq!(
            decode_component(&bytes, WorldTick(tick.0 - 1)),
            Err(WorldError::InvalidValue)
        );
        let due = due_events(&state, WorldTick(u64::MAX)).unwrap();
        assert!(
            apply_due(&state, WorldTick(u64::MAX), &due)
                .unwrap()
                .schedules()
                .is_empty()
        );
    }
    assert!(validate_at(&support::state(), WorldTick(u64::MAX)).is_ok());
}
#[test]
fn action_output_allows_due_now_only_until_exact_end_of_tick_reduction() {
    let state = scheduled(&support::state(), WorldTick(1), false);
    let faction = state.factions().iter().find(|f| f.ordinal == 1).unwrap().id;
    let mut c = support::colony(&state, 3, 8);
    c.action = WorldAction::Travel {
        fleet: state
            .fleets()
            .iter()
            .find(|f| f.faction == faction)
            .unwrap()
            .id,
        destination: state.systems().iter().find(|s| s.ordinal == 2).unwrap().id,
    };
    let plan = evaluate(&state, WorldTick(3), &c).unwrap();
    let intermediate = apply_plan(&state, WorldTick(3), &plan).unwrap();
    assert_eq!(intermediate.schedules().len(), 2);
    assert_eq!(intermediate.schedules()[0].due, WorldTick(3));
    assert_eq!(intermediate.schedules()[1].due, WorldTick(6));
    assert_eq!(
        encode_component(&intermediate, WorldTick(3)),
        Err(WorldError::InvalidValue)
    );
    let due = due_events(&intermediate, WorldTick(3)).unwrap();
    let committed = apply_due(&intermediate, WorldTick(3), &due).unwrap();
    assert_eq!(committed.schedules().len(), 1);
    assert_eq!(committed.schedules()[0].due, WorldTick(6));
    assert_eq!(committed.revision(), AggregateRevision(3));
    assert_eq!(
        decode_component(
            &encode_component(&committed, WorldTick(3)).unwrap(),
            WorldTick(3)
        )
        .unwrap(),
        committed
    );
}
