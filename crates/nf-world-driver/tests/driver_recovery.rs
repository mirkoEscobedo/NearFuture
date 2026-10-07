mod driver_support;
use nf_contract::identity::*;
use nf_store::miniature::{MiniatureCancelCause, MiniatureRequestStatus};
use nf_world_driver::{ActionRequest, ActionSelection, Driver, OpenOptions, SignerOptions};
fn request(n: u8) -> ActionRequest {
    ActionRequest {
        request: RequestId::from_bytes([n; 16]),
        operation: OperationId::from_bytes([n + 1; 16]),
        job: JobId::from_bytes([n + 2; 16]),
        action: ActionSelection::Colony {
            site: 1,
            faction: 0,
        },
    }
}
#[test]
fn restarted_pending_needs_fresh_original_actor_proofs_or_explicit_owner_tick_neutral_cancel() {
    let fixture = driver_support::Fixture::new();
    let mut driver = Driver::create(fixture.options.clone()).unwrap();
    driver.claim_authority().unwrap();
    let action = request(40);
    let original = driver.prepare_action(&action).unwrap();
    let old = driver.status().unwrap();
    drop(driver);
    let open = OpenOptions {
        database: fixture.options.database.clone(),
        scope: fixture.options.policy.scope,
        minimum_event: old.known.storage.event_sequence,
        minimum_store: old.known.storage.store_revision,
        minimum_membership: 2,
        minimum_term: old.known.minimum_authority_term,
    };
    let signer = SignerOptions {
        vault: fixture.options.vault.clone(),
        game_save_root: fixture.options.game_save_root.clone(),
        account: fixture.options.policy.owner.account,
        device: fixture.options.policy.owner.device,
    };
    let mut reopened = Driver::open(open, signer).unwrap();
    assert_eq!(reopened.status().unwrap(), old);
    assert!(reopened.advance_pending(true).is_err());
    reopened.claim_authority().unwrap();
    assert!(reopened.advance_pending(true).is_err());
    reopened
        .resume_pending()
        .expect("actual fresh original actor proof reauthorizes unchanged intent");
    let retained = reopened.prepare_action(&action).unwrap();
    assert_eq!(retained.intent, original.intent);
    assert_eq!(retained.binding_digest, original.binding_digest);
    reopened.advance_pending(true).unwrap();
    assert_eq!(
        reopened.status().unwrap().world.metadata().tick,
        WorldTick(1)
    );
    let mut later = request(50);
    later.action = ActionSelection::Build {
        market: 0,
        kind: nf_world::IndustryKind::Farming,
    };
    reopened.prepare_action(&later).unwrap();
    let before = reopened.status().unwrap();
    reopened
        .cancel_pending(MiniatureCancelCause::Cancelled)
        .expect("guarded owner cancellation commits no strategic tick");
    let after = reopened.status().unwrap();
    assert_eq!(after.world.metadata().tick, before.world.metadata().tick);
    assert_eq!(after.world.component(), before.world.component());
    assert_eq!(
        after.world.metadata().event_sequence.0,
        before.world.metadata().event_sequence.0 + 1
    );
    assert!(after.pending.is_none());
    let status = reopened.prepare_action(&later).unwrap();
    assert!(matches!(
        status.status,
        MiniatureRequestStatus::Committed {
            rejection: Some(nf_kernel::miniature::MiniatureRejection::Cancelled),
            ..
        }
    ));
    assert_eq!(reopened.status().unwrap(), after);
}
