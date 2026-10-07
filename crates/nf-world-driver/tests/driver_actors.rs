mod driver_support;
use nf_contract::identity::*;
use nf_identity::private_storage::PrivateVault;
use nf_world_driver::{
    ActionRequest, ActionSelection, ActorRequest, Driver, OpenOptions, SignerOptions,
};
#[test]
fn bounded_existing_controller_keys_prepare_sorted_frontier_and_restart_requires_every_original_actor()
 {
    let f = driver_support::Fixture::new();
    let o = &f.options;
    let vault = PrivateVault::open(&f.scratch.0.join("private1"), &o.game_save_root).unwrap();
    let actor = vault.load_identity(vec![2]).unwrap().public;
    let extra = SignerOptions {
        vault: f.scratch.0.join("private1"),
        game_save_root: o.game_save_root.clone(),
        account: actor.account,
        device: actor.device,
    };
    let primary = SignerOptions {
        vault: o.vault.clone(),
        game_save_root: o.game_save_root.clone(),
        account: o.policy.owner.account,
        device: o.policy.owner.device,
    };
    let mut driver = Driver::create(o.clone()).unwrap();
    driver.claim_authority().unwrap();
    driver
        .configure_signer(extra.clone())
        .expect("only existing current admitted controller keys");
    assert!(driver.configure_signer(extra.clone()).is_err());
    assert!(driver.configure_signer(primary.clone()).is_err());
    let requests = vec![
        ActorRequest {
            account: actor.account,
            device: actor.device,
            action: ActionRequest {
                request: RequestId::from_bytes([40; 16]),
                operation: OperationId::from_bytes([41; 16]),
                job: JobId::from_bytes([42; 16]),
                action: ActionSelection::Travel {
                    faction: 1,
                    destination: 2,
                },
            },
        },
        ActorRequest {
            account: primary.account,
            device: primary.device,
            action: ActionRequest {
                request: RequestId::from_bytes([50; 16]),
                operation: OperationId::from_bytes([51; 16]),
                job: JobId::from_bytes([52; 16]),
                action: ActionSelection::Build {
                    market: 0,
                    kind: nf_world::IndustryKind::Farming,
                },
            },
        },
    ];
    driver
        .prepare_frontier(&requests)
        .expect("current signatures admit all actors before canonical resource planning");
    let pending = driver.status().unwrap();
    assert_eq!(pending.pending.as_ref().unwrap().intents().len(), 2);
    assert!(pending.pending.as_ref().unwrap().reservation().is_some());
    assert!(driver.prepare_frontier(&requests).is_err());
    assert_eq!(driver.status().unwrap(), pending);
    drop(driver);
    let open = OpenOptions {
        database: o.database.clone(),
        scope: o.policy.scope,
        minimum_event: pending.known.storage.event_sequence,
        minimum_store: pending.known.storage.store_revision,
        minimum_membership: 2,
        minimum_term: pending.known.minimum_authority_term,
    };
    let mut reopened = Driver::open(open, primary).unwrap();
    reopened.claim_authority().unwrap();
    let before = reopened.status().unwrap();
    assert!(reopened.resume_pending().is_err());
    assert_eq!(reopened.status().unwrap(), before);
    reopened.configure_signer(extra).unwrap();
    reopened
        .resume_pending()
        .expect("complete unchanged original actor/device proofs");
    reopened.advance_pending(true).unwrap();
    assert_eq!(
        reopened.status().unwrap().world.metadata().tick,
        WorldTick(1)
    );
}
