mod driver_support;
use nf_contract::identity::*;
use nf_world_driver::{Driver, OpenOptions, SignerOptions};
#[test]
fn read_only_status_and_signer_reopen_retain_genesis_without_authority_or_keys_in_save() {
    let fixture = driver_support::Fixture::new();
    let created = Driver::create(fixture.options.clone()).unwrap();
    let initial = created.status().unwrap();
    drop(created);
    let options = OpenOptions {
        database: fixture.options.database.clone(),
        scope: fixture.options.policy.scope,
        minimum_event: EventSeq(0),
        minimum_store: 0,
        minimum_membership: 2,
        minimum_term: AuthorityTerm(0),
    };
    assert_eq!(
        Driver::read_status(options.clone()).expect("real readonly reopen"),
        initial
    );
    let signer = SignerOptions {
        vault: fixture.options.vault.clone(),
        game_save_root: fixture.options.game_save_root.clone(),
        account: fixture.options.policy.owner.account,
        device: fixture.options.policy.owner.device,
    };
    let reopened = Driver::open(options.clone(), signer)
        .expect("actual persisted identity peer selects existing local signer");
    assert_eq!(reopened.local_identity(), &fixture.options.policy.owner);
    assert_eq!(reopened.status().unwrap(), initial);
    drop(reopened);
    let mut protected = options;
    protected.minimum_membership = 3;
    assert!(Driver::read_status(protected).is_err());
}
