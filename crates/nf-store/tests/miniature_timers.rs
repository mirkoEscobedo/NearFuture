#[path = "miniature_support/flow.rs"]
mod flow;
mod miniature_support;
use miniature_support::*;
use nf_kernel::miniature::*;
use nf_store::miniature::*;
use nf_world::*;
fn action(store: &MiniatureStore, f: &Fixture, tick: u8) -> Vec<MiniatureIntent> {
    let mut i = intent(store.world(), &f.policy.owner, tick);
    let w = store.world().component();
    let faction = w
        .factions()
        .iter()
        .find(|x| x.account == f.policy.owner.account)
        .unwrap();
    match tick {
        1 => {
            i.action = WorldAction::BuildIndustry {
                market: w
                    .markets()
                    .iter()
                    .find(|m| m.owner == Some(faction.id))
                    .unwrap()
                    .id,
                kind: IndustryKind::Farming,
            }
        }
        2 => {
            let fleet = w.fleets().iter().find(|v| v.faction == faction.id).unwrap();
            let FleetLocation::Docked(origin) = fleet.location else {
                panic!("initial dock")
            };
            i.action = WorldAction::Travel {
                fleet: fleet.id,
                destination: w.systems().iter().find(|s| s.id != origin).unwrap().id,
            };
        }
        _ => return vec![],
    }
    vec![i]
}
fn assert_timing(store: &MiniatureStore, f: &Fixture, tick: u8) {
    let w = store.world().component();
    let faction = w
        .factions()
        .iter()
        .find(|x| x.account == f.policy.owner.account)
        .unwrap();
    assert_eq!(faction.credits, 140);
    assert_eq!(faction.supplies, if tick == 1 { 80 } else { 60 });
    assert_eq!(faction.spent_credits, 60);
    assert_eq!(faction.spent_supplies, if tick == 1 { 20 } else { 40 });
    let slot = w
        .markets()
        .iter()
        .find(|m| m.owner == Some(faction.id))
        .unwrap()
        .industries[0];
    if tick < 3 {
        assert!(matches!(slot, IndustryState::Constructing(_)));
        assert!(w.schedules().iter().any(|s| s.due.0 == 3));
    } else {
        assert_eq!(slot, IndustryState::Complete);
    }
    let fleet = w.fleets().iter().find(|v| v.faction == faction.id).unwrap();
    if (2..5).contains(&tick) {
        assert!(matches!(fleet.location, FleetLocation::InTransit(_)));
        assert!(w.schedules().iter().any(|s| s.due.0 == 5));
    }
    if tick >= 5 {
        assert!(matches!(fleet.location, FleetLocation::Docked(_)));
        assert!(w.schedules().is_empty());
    }
    assert_eq!(store.world().metadata().tick.0, u64::from(tick));
    assert_eq!(
        store.world().metadata().provider_revision.0,
        u64::from(tick)
    );
    assert_eq!(
        w.revision().0,
        match tick {
            1 => 1,
            2 => 2,
            3 | 4 => 3,
            _ => 4,
        }
    );
}
#[test]
fn construction_and_arrival_restart_before_at_after_due_match_uninterrupted_world_bytes() {
    let left = Scratch::new();
    let right = Scratch::new();
    let mut f = Fixture::new();
    let mut continuous = MiniatureStore::create(
        left.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    let mut restarted = MiniatureStore::create(
        right.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    claim(&mut continuous, &mut f);
    claim(&mut restarted, &mut f);
    for tick in 1..=6 {
        let a = action(&continuous, &f, tick);
        let b = action(&restarted, &f, tick);
        flow::prepare(&mut continuous, &mut f, a);
        flow::prepare(&mut restarted, &mut f, b);
        restarted = flow::reopen(restarted, &mut f, &right.db());
        flow::advance(&mut continuous, &mut f);
        flow::advance(&mut restarted, &mut f);
        assert_timing(&continuous, &f, tick);
        assert_timing(&restarted, &f, tick);
        assert_eq!(
            encode_miniature_snapshot(continuous.world()).unwrap(),
            encode_miniature_snapshot(restarted.world()).unwrap()
        );
        restarted = flow::reopen(restarted, &mut f, &right.db());
        assert_eq!(continuous.world().outcomes(), restarted.world().outcomes());
    }
}
#[test]
fn checkpoint_and_tail_replay_restore_exact_empty_tick_state_without_runtime_activity() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let mut store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    claim(&mut store, &mut f);
    for _ in 0..65 {
        flow::prepare(&mut store, &mut f, vec![]);
        flow::advance(&mut store, &mut f);
    }
    assert_eq!(store.world().metadata().tick.0, 65);
    assert_eq!(store.world().component().revision().0, 0);
    let known = store.known_frontiers().unwrap();
    let bytes = store.snapshot_bytes().unwrap();
    drop(store);
    let mut reopened = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(reopened.snapshot_bytes().unwrap(), bytes);
    claim(&mut reopened, &mut f);
    flow::prepare(&mut reopened, &mut f, vec![]);
    let batch = settle_miniature(
        reopened.world(),
        reopened.pending().unwrap(),
        reopened.authority().unwrap().context(),
    )
    .unwrap();
    let proof = flow::attempt(&mut reopened, &mut f, ChallengeRequest::Commit);
    assert_eq!(
        reopened.commit(&batch, proof),
        Err(MiniatureStoreError::NoActivity)
    );
}
