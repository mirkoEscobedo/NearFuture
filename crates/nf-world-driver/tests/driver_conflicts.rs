mod driver_support;
use nf_contract::identity::*;
use nf_identity::private_storage::PrivateVault;
use nf_kernel::miniature::MiniatureRejection;
use nf_world_driver::{ActionRequest, ActionSelection, ActorRequest, Driver, SignerOptions};
fn action(n: u8, selection: ActionSelection) -> ActionRequest {
    ActionRequest {
        request: RequestId::from_bytes([n; 16]),
        operation: OperationId::from_bytes([n; 16]),
        job: JobId::from_bytes([n; 16]),
        action: selection,
    }
}
#[test]
fn real_authenticated_simultaneous_ownership_conflicts_choose_first_legal_cost_and_insufficient_resources_do_not_debit()
 {
    let f = driver_support::Fixture::new();
    let o = &f.options;
    let actor = PrivateVault::open(&f.scratch.0.join("private1"), &o.game_save_root)
        .unwrap()
        .load_identity(vec![2])
        .unwrap()
        .public;
    let mut driver = Driver::create(o.clone()).unwrap();
    driver.claim_authority().unwrap();
    driver
        .configure_signer(SignerOptions {
            vault: f.scratch.0.join("private1"),
            game_save_root: o.game_save_root.clone(),
            account: actor.account,
            device: actor.device,
        })
        .unwrap();
    let selection = ActionSelection::Build {
        market: 2,
        kind: nf_world::IndustryKind::Farming,
    };
    let mut requests = vec![
        ActorRequest {
            account: actor.account,
            device: actor.device,
            action: action(3, selection.clone()),
        },
        ActorRequest {
            account: o.policy.owner.account,
            device: o.policy.owner.device,
            action: action(1, selection.clone()),
        },
        ActorRequest {
            account: actor.account,
            device: actor.device,
            action: action(2, selection),
        },
    ];
    driver.prepare_frontier(&requests).unwrap();
    let pending = driver.status().unwrap();
    let frontier = pending.pending.unwrap();
    let hold = frontier.reservation().unwrap();
    assert_eq!(hold.operation(), OperationId::from_bytes([2; 16]));
    assert_eq!((hold.credits(), hold.supplies()), (60, 20));
    driver.advance_pending(true).unwrap();
    let committed = driver.status().unwrap();
    let faction = committed
        .world
        .component()
        .factions()
        .iter()
        .find(|f| f.ordinal == 1)
        .unwrap();
    assert_eq!((faction.credits, faction.supplies), (140, 80));
    assert!(
        committed
            .world
            .outcomes()
            .iter()
            .any(|o| o.operation == OperationId::from_bytes([1; 16])
                && o.rejection == Some(MiniatureRejection::Unauthorized))
    );
    assert!(
        committed
            .world
            .outcomes()
            .iter()
            .any(|o| o.operation == OperationId::from_bytes([3; 16])
                && o.rejection == Some(MiniatureRejection::Conflict))
    );
    requests.clear();
    for (n, selection) in [
        (
            10,
            ActionSelection::Colony {
                site: 1,
                faction: 0,
            },
        ),
        (
            20,
            ActionSelection::Build {
                market: 0,
                kind: nf_world::IndustryKind::Farming,
            },
        ),
    ] {
        let request = action(n, selection);
        driver.prepare_action(&request).unwrap();
        driver.advance_pending(true).unwrap();
    }
    let request = action(
        30,
        ActionSelection::Build {
            market: 0,
            kind: nf_world::IndustryKind::Workshop,
        },
    );
    let before = driver.status().unwrap();
    driver.prepare_action(&request).unwrap();
    assert!(
        driver
            .status()
            .unwrap()
            .pending
            .unwrap()
            .reservation()
            .is_none()
    );
    driver.advance_pending(true).unwrap();
    let after = driver.status().unwrap();
    assert_eq!(
        after.world.component().factions(),
        before.world.component().factions()
    );
    assert!(
        after
            .world
            .outcomes()
            .iter()
            .any(|o| o.operation == request.operation
                && o.rejection == Some(MiniatureRejection::Resources))
    );
}
