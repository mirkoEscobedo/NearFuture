mod driver_support;
use nf_contract::identity::*;
use nf_world_driver::{ActionRequest, ActionSelection, Driver, OpenOptions, SignerOptions};
fn reopen(f: &driver_support::Fixture, driver: Driver) -> Driver {
    let old = driver.status().unwrap();
    drop(driver);
    let o = OpenOptions {
        database: f.options.database.clone(),
        scope: f.options.policy.scope,
        minimum_event: old.known.storage.event_sequence,
        minimum_store: old.known.storage.store_revision,
        minimum_membership: 2,
        minimum_term: old.known.minimum_authority_term,
    };
    let s = SignerOptions {
        vault: f.options.vault.clone(),
        game_save_root: f.options.game_save_root.clone(),
        account: f.options.policy.owner.account,
        device: f.options.policy.owner.device,
    };
    let mut reopened = Driver::open(o, s).unwrap();
    assert_eq!(
        reopened.status().unwrap(),
        old,
        "reopen neither advances nor catches up wall time"
    );
    assert!(
        reopened.advance_empty(true).is_err(),
        "old SID cannot authorize a reopened runtime"
    );
    reopened.claim_authority().unwrap();
    assert_eq!(reopened.status().unwrap().world, old.world);
    reopened
}
fn commit(driver: &mut Driver, n: u8, action: ActionSelection) {
    let request = ActionRequest {
        request: RequestId::from_bytes([n; 16]),
        operation: OperationId::from_bytes([n + 1; 16]),
        job: JobId::from_bytes([n + 2; 16]),
        action,
    };
    driver.prepare_action(&request).unwrap();
    driver.advance_pending(true).unwrap();
}
#[test]
fn true_sql_restart_before_at_and_after_due_reproduces_uninterrupted_exact_canonical_world() {
    let f = driver_support::Fixture::new();
    let mut reference_options = f.options.clone();
    reference_options.database = f.scratch.0.join("reference.sqlite");
    let mut reference = Driver::create(reference_options).unwrap();
    reference.claim_authority().unwrap();
    let mut restarted = Driver::create(f.options.clone()).unwrap();
    restarted.claim_authority().unwrap();
    for driver in [&mut reference, &mut restarted] {
        commit(
            driver,
            40,
            ActionSelection::Build {
                market: 0,
                kind: nf_world::IndustryKind::Farming,
            },
        );
        assert_eq!(
            driver.status().unwrap().world.component().schedules()[0].due,
            WorldTick(3)
        );
        commit(
            driver,
            50,
            ActionSelection::Travel {
                faction: 0,
                destination: 1,
            },
        );
        let state = driver.status().unwrap();
        assert!(
            state
                .world
                .component()
                .schedules()
                .iter()
                .any(|s| s.due == WorldTick(5))
        );
    }
    restarted = reopen(&f, restarted); // N2, immediately before industry completion.
    reference.advance_empty(true).unwrap();
    restarted.advance_empty(true).unwrap();
    assert_eq!(
        restarted.status().unwrap().world,
        reference.status().unwrap().world
    );
    assert_eq!(
        restarted
            .status()
            .unwrap()
            .world
            .component()
            .schedules()
            .len(),
        1
    );
    restarted = reopen(&f, restarted); // N3, industry completion has already committed.
    for _ in 0..2 {
        reference.advance_empty(true).unwrap();
        restarted.advance_empty(true).unwrap();
    }
    restarted = reopen(&f, restarted); // N5, arrival completion has committed.
    let actual = restarted.status().unwrap().world;
    let expected = reference.status().unwrap().world;
    assert_eq!(actual.metadata().tick, WorldTick(5));
    assert!(actual.component().schedules().is_empty());
    assert_eq!(actual, expected);
    assert_eq!(
        nf_kernel::miniature::encode_miniature_snapshot(&actual).unwrap(),
        nf_kernel::miniature::encode_miniature_snapshot(&expected).unwrap()
    );
    let faction = actual
        .component()
        .factions()
        .iter()
        .find(|f| f.ordinal == 0)
        .unwrap();
    assert_eq!(
        (
            faction.credits,
            faction.supplies,
            faction.spent_credits,
            faction.spent_supplies
        ),
        (140, 60, 60, 40)
    );
}
