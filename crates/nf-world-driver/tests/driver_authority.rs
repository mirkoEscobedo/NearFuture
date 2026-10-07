mod driver_support;
use nf_contract::identity::*;
use nf_world_driver::{Driver, OpenOptions, SignerOptions};
#[test]
fn explicit_real_claim_and_reopen_claim_use_greater_terms_fresh_sessions_without_strategic_ticks() {
    let fixture = driver_support::Fixture::new();
    let mut driver = Driver::create(fixture.options.clone()).unwrap();
    driver
        .claim_authority()
        .expect("guarded signed authority claim");
    let first = driver.status().unwrap();
    let authority = first.authority.unwrap();
    assert_eq!(authority.term, AuthorityTerm(1));
    assert_ne!(authority.session, RuntimeSession(0));
    assert_eq!(first.world.metadata().tick, WorldTick(0));
    assert_eq!(first.known.storage.store_revision, 1);
    assert_eq!(first.world.metadata().event_sequence, EventSeq(0));
    drop(driver);
    let options = OpenOptions {
        database: fixture.options.database.clone(),
        scope: fixture.options.policy.scope,
        minimum_event: EventSeq(0),
        minimum_store: 1,
        minimum_membership: 2,
        minimum_term: AuthorityTerm(1),
    };
    let signer = SignerOptions {
        vault: fixture.options.vault.clone(),
        game_save_root: fixture.options.game_save_root.clone(),
        account: fixture.options.policy.owner.account,
        device: fixture.options.policy.owner.device,
    };
    let mut reopened = Driver::open(options, signer).unwrap();
    assert_eq!(reopened.status().unwrap(), first);
    reopened.claim_authority().unwrap();
    let second = reopened.status().unwrap();
    let next = second.authority.unwrap();
    assert_eq!(next.term, AuthorityTerm(2));
    assert_ne!(next.session, authority.session);
    assert_eq!(second.world, first.world);
    assert_eq!(second.known.storage.store_revision, 2);
}
