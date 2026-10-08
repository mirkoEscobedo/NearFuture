mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{Burn, BurnReason, policy_digest};
use nf_store::supplies::{ChallengeRequest, SuppliesStore};
use supplies_support::{Fixture, Scratch};

#[test]
fn authorized_burn_is_durable_once_across_retry_restart_and_checkpoint() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual retained authorized grant");
    let burn = Burn {
        request: RequestId::from_bytes([11; 16]),
        burn: OperationId::from_bytes([12; 16]),
        actor: fixture.issuer.account,
        device: fixture.issuer.device,
        owner: fixture.beneficiary.account,
        universe: fixture.policy.universe,
        history: fixture.policy.history,
        policy: policy_digest(&fixture.policy).unwrap(),
        content: issuance.content,
        origin: issuance.origin,
        reason: BurnReason::AuthorityDestruction,
        amount: 10,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Burn(&burn));
    let original = store.burn(&burn, proof).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Burn(&burn));
    let retry = store.burn(&burn, proof).unwrap();
    let mut duplicate = burn;
    duplicate.request = RequestId::from_bytes([13; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Burn(&duplicate));
    let repeated_burn = store.burn(&duplicate, proof).unwrap();
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let query = fixture.query();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&query));
    let balance = store.balance(&query, proof).unwrap();
    // Authenticated setup must complete before the literal missing-effect oracle.
    assert_eq!(
        balance.available, 15,
        "one authorized burn must remove ten spendable units once"
    );
    assert_eq!((balance.minted, balance.burned), (25, 10));
    assert_eq!(
        (balance.reserved, balance.pending, balance.externalized),
        (0, 0, 0)
    );
    let original = original.expect("authorized burn must retain a terminal outcome");
    assert_eq!((original.burn, original.revision), (burn.burn, 2));
    assert_eq!(retry, Some(original));
    assert_eq!(repeated_burn, Some(original));
}
