mod registration_support;

use nf_store::registration::{BranchRegistrar, CampaignBinding, RegisteredBranch};
use registration_support::Fixture;

#[test]
fn signed_branch_registration_is_durable_once_across_retry_and_reopen() {
    let fixture = Fixture::new();
    let mut registrar = fixture.create();
    let request = fixture.request();
    let genesis = registrar
        .known_frontier()
        .expect("actual registrar genesis");
    assert_eq!(
        (genesis.revision, genesis.minimum_membership_revision),
        (0, 1)
    );
    assert_eq!(genesis.head, [0; 32]);

    let proof = fixture.attempt(&mut registrar, &request);
    let registered = registrar
        .register_branch(&request, proof)
        .expect("authenticated branch registration must commit its exact receipt");
    let expected = RegisteredBranch {
        binding: CampaignBinding {
            scope: request.scope,
            campaign: request.campaign,
            branch: request.branch,
            account: request.account,
        },
        original_request: request.request,
        revision: 1,
    };
    assert_eq!(registered, expected);
    let once = registrar
        .known_frontier()
        .expect("committed registration frontier");
    assert_eq!((once.revision, once.minimum_membership_revision), (1, 1));
    assert_ne!(once.head, genesis.head);

    let retry = fixture.attempt(&mut registrar, &request);
    assert_eq!(
        registrar.register_branch(&request, retry).unwrap(),
        expected
    );
    assert_eq!(registrar.known_frontier().unwrap(), once);
    drop(registrar);

    let mut registrar = BranchRegistrar::open_existing(fixture.db(), &fixture.policy, once)
        .expect("reopen the actual committed registration profile");
    assert_eq!(registrar.known_frontier().unwrap(), once);
    let retry = fixture.attempt(&mut registrar, &request);
    assert_eq!(
        registrar.register_branch(&request, retry).unwrap(),
        expected
    );
    assert_eq!(registrar.known_frontier().unwrap(), once);
}
