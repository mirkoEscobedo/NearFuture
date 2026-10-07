use nf_contract::identity::*;
use nf_world::*;
fn state() -> State {
    generate(Genesis {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
        seed: [3; 32],
        accounts: [
            AccountId::from_bytes([4; 16]),
            AccountId::from_bytes([5; 16]),
            AccountId::from_bytes([6; 16]),
        ],
    })
    .unwrap()
}
#[test]
fn colony_plan_reserves_exact_resources_and_changes_one_owner_once() {
    let s = state();
    let site = s.markets().iter().find(|m| m.ordinal == 1).unwrap().id;
    let faction = s.factions().iter().find(|f| f.ordinal == 0).unwrap();
    let c = Candidate {
        request: RequestId::from_bytes([8; 16]),
        operation: OperationId::from_bytes([9; 16]),
        job: JobId::from_bytes([10; 16]),
        actor: faction.account,
        expected_revision: AggregateRevision(0),
        action: WorldAction::CreateColony {
            site,
            faction: faction.id,
        },
    };
    let p = evaluate(&s, WorldTick(1), &c).expect("legal colony plan");
    assert_eq!(
        (p.reservation().credits(), p.reservation().supplies()),
        (100, 40)
    );
    let next = apply_plan(&s, p.committing_tick(), &p).unwrap();
    let owner = next.factions().iter().find(|f| f.id == faction.id).unwrap();
    assert_eq!(
        (
            owner.credits,
            owner.supplies,
            owner.spent_credits,
            owner.spent_supplies
        ),
        (100, 60, 100, 40)
    );
    assert_eq!(
        next.markets().iter().find(|m| m.id == site).unwrap().owner,
        Some(faction.id)
    );
    assert_eq!(next.revision(), AggregateRevision(1));
    assert_eq!(
        apply_plan(&next, p.committing_tick(), &p),
        Err(WorldError::StaleRevision)
    );
}
