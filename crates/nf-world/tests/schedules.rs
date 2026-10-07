mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn build_duration_starts_at_committing_tick_and_completion_is_one_use() {
    let s = support::state();
    let f = s.factions().iter().find(|f| f.ordinal == 0).unwrap();
    let market = s.markets().iter().find(|m| m.ordinal == 0).unwrap().id;
    let c = Candidate {
        request: RequestId::from_bytes([7; 16]),
        operation: OperationId::from_bytes([8; 16]),
        job: JobId::from_bytes([9; 16]),
        actor: f.account,
        expected_revision: s.revision(),
        action: WorldAction::BuildIndustry {
            market,
            kind: IndustryKind::Farming,
        },
    };
    let p = evaluate(&s, WorldTick(10), &c).expect("supported industry build");
    let started = apply_plan(&s, p.committing_tick(), &p).unwrap();
    assert_eq!(started.schedules()[0].due, WorldTick(12));
    assert_eq!(
        (p.reservation().credits(), p.reservation().supplies()),
        (60, 20)
    );
    assert!(due_events(&started, WorldTick(11)).unwrap().is_empty());
    let due = due_events(&started, WorldTick(12)).unwrap();
    assert_eq!(due.len(), 1);
    let complete = apply_due(&started, WorldTick(12), &due).unwrap();
    assert!(complete.schedules().is_empty());
    assert_eq!(
        complete
            .markets()
            .iter()
            .find(|m| m.id == market)
            .unwrap()
            .industries[0],
        IndustryState::Complete
    );
    assert!(apply_due(&complete, WorldTick(12), &due).is_err());
    let overflow = evaluate(&s, WorldTick(u64::MAX), &c);
    assert_eq!(overflow, Err(WorldError::Overflow));
}
