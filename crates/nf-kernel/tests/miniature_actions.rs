mod miniature_support;
use nf_contract::identity::*;
use nf_kernel::{AuthorityContext, miniature::*};
use nf_world::{IndustryKind, IndustryState, WorldAction};
fn step(
    world: &MiniatureWorld,
    operation: u8,
    action: Option<(usize, WorldAction)>,
) -> (MiniatureWorld, MiniatureBatch) {
    let intents = action
        .map(|(account, action)| {
            let mut intent = miniature_support::intent(world);
            intent.request = RequestId::from_bytes([operation; 16]);
            intent.operation = OperationId::from_bytes([operation; 16]);
            intent.job = JobId::from_bytes([operation; 16]);
            intent.actor = world.component().genesis().accounts[account];
            intent.action = action;
            vec![intent]
        })
        .unwrap_or_default();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let frontier = admit_miniature(world, intents, authority, 2).unwrap();
    let batch = settle_miniature(world, &frontier, authority).unwrap();
    let bytes = encode_miniature_batch(&batch).unwrap();
    assert_eq!(decode_miniature_batch(&bytes, world).unwrap(), batch);
    let next = replay_miniature(world, &batch).unwrap();
    assert_eq!(
        decode_miniature_snapshot(&encode_miniature_snapshot(&next).unwrap()).unwrap(),
        next
    );
    (next, batch)
}
#[test]
fn recorded_costs_schedules_and_action_with_due_timer_replay_after_typed_checkpoint() {
    let genesis = miniature_support::world();
    let faction = genesis
        .component()
        .factions()
        .iter()
        .find(|f| f.ordinal == 0)
        .unwrap()
        .id;
    let other = genesis
        .component()
        .factions()
        .iter()
        .find(|f| f.ordinal == 1)
        .unwrap()
        .id;
    let site = genesis
        .component()
        .markets()
        .iter()
        .find(|m| m.ordinal == 1)
        .unwrap()
        .id;
    let fleet = genesis
        .component()
        .fleets()
        .iter()
        .find(|f| f.faction == faction)
        .unwrap()
        .id;
    let destination = genesis
        .component()
        .systems()
        .iter()
        .find(|s| s.ordinal == 1)
        .unwrap()
        .id;
    let (first, colony) = step(
        &genesis,
        40,
        Some((0, WorldAction::CreateColony { site, faction })),
    );
    assert_eq!(
        (
            colony.action_event().unwrap().cost().credits(),
            colony.action_event().unwrap().cost().supplies()
        ),
        (100, 40)
    );
    let (second, build) = step(
        &first,
        41,
        Some((
            0,
            WorldAction::BuildIndustry {
                market: site,
                kind: IndustryKind::Farming,
            },
        )),
    );
    assert_eq!(
        build
            .action_event()
            .unwrap()
            .created_schedule()
            .unwrap()
            .due,
        WorldTick(4)
    );
    let checkpoint = encode_miniature_snapshot(&second).unwrap();
    let restored = decode_miniature_snapshot(&checkpoint).unwrap();
    let (third, travel) = step(
        &restored,
        42,
        Some((0, WorldAction::Travel { fleet, destination })),
    );
    assert_eq!(
        travel
            .action_event()
            .unwrap()
            .created_schedule()
            .unwrap()
            .due,
        WorldTick(6)
    );
    let (fourth, relation) = step(
        &third,
        43,
        Some((
            0,
            WorldAction::SetRelationship {
                faction,
                other,
                score: 123,
            },
        )),
    );
    assert_eq!(relation.due_events().len(), 1);
    assert!(relation.action_event().is_some());
    assert_eq!(fourth.component().revision(), AggregateRevision(5));
    assert_eq!(fourth.metadata().tick, WorldTick(4));
    assert_eq!(
        fourth
            .component()
            .markets()
            .iter()
            .find(|m| m.id == site)
            .unwrap()
            .industries[0],
        IndustryState::Complete
    );
    let resources = fourth
        .component()
        .factions()
        .iter()
        .find(|f| f.id == faction)
        .unwrap();
    assert_eq!(
        (
            resources.credits,
            resources.supplies,
            resources.spent_credits,
            resources.spent_supplies
        ),
        (40, 20, 160, 80)
    );
    let mut wrong_cost = encode_miniature_batch(&colony).unwrap();
    wrong_cost[490..498].copy_from_slice(&99u64.to_le_bytes());
    assert!(decode_miniature_batch(&wrong_cost, &genesis).is_err());
    let mut wrong_outcome = encode_miniature_batch(&colony).unwrap();
    *wrong_outcome.last_mut().unwrap() = MiniatureRejection::Conflict as u8;
    assert!(decode_miniature_batch(&wrong_outcome, &genesis).is_err());
    assert_eq!(replay_miniature(&third, &relation).unwrap(), fourth);
}
#[test]
fn due_set_is_complete_ordered_and_exact_recorded_bytes_not_just_matching_after_hash() {
    let genesis = miniature_support::world();
    let faction = genesis
        .component()
        .factions()
        .iter()
        .find(|f| f.ordinal == 0)
        .unwrap()
        .id;
    let fleet = genesis
        .component()
        .fleets()
        .iter()
        .find(|f| f.faction == faction)
        .unwrap()
        .id;
    let destination = genesis
        .component()
        .systems()
        .iter()
        .find(|s| s.ordinal == 1)
        .unwrap()
        .id;
    let market = genesis
        .component()
        .markets()
        .iter()
        .find(|m| m.ordinal == 2)
        .unwrap()
        .id;
    let (first, _) = step(
        &genesis,
        50,
        Some((0, WorldAction::Travel { fleet, destination })),
    );
    let (second, _) = step(
        &first,
        51,
        Some((
            1,
            WorldAction::BuildIndustry {
                market,
                kind: IndustryKind::Workshop,
            },
        )),
    );
    let (third, _) = step(&second, 0, None);
    let (fourth, timers) = step(&third, 0, None);
    assert_eq!(timers.due_events().len(), 2);
    assert_eq!(fourth.component().revision(), AggregateRevision(4));
    let original = encode_miniature_batch(&timers).unwrap();
    let lengths: Vec<_> = timers
        .due_events()
        .iter()
        .map(|s| match s.kind {
            nf_world::ScheduleKind::Industry { .. } => 58,
            nf_world::ScheduleKind::Arrival { .. } => 89,
        })
        .collect();
    let start = 131;
    let split = start + lengths[0];
    let end = split + lengths[1];
    let mut reordered = original.clone();
    reordered.splice(
        start..end,
        original[split..end]
            .iter()
            .chain(original[start..split].iter())
            .copied(),
    );
    assert!(decode_miniature_batch(&reordered, &third).is_err());
    let mut missing = original.clone();
    missing.drain(start..split);
    missing[127..131].copy_from_slice(&1u32.to_le_bytes());
    assert!(decode_miniature_batch(&missing, &third).is_err());
    let mut false_deadline = original;
    false_deadline[start + 32..start + 40].copy_from_slice(&5u64.to_le_bytes());
    assert!(decode_miniature_batch(&false_deadline, &third).is_err());
}
