mod supplies_support;
use nf_contract::identity::RequestId;
use nf_store::supplies::{ChallengeRequest, SuppliesStore};
use supplies_support::{Fixture, Scratch};

#[test]
fn authorized_issuance_is_durable_once_across_retry_restart_and_compaction() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let original = store.issue(&issuance, proof).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let retry = store.issue(&issuance, proof).unwrap();
    let mut duplicate = issuance;
    duplicate.request = RequestId::from_bytes([10; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&duplicate));
    let repeated_issuance = store.issue(&duplicate, proof).unwrap();
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let query = fixture.query();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&query));
    let balance = store.balance(&query, proof).unwrap();
    // All signed setup/recovery must succeed before the literal missing-behavior oracle.
    assert_eq!(
        balance.available, 25,
        "one authorized issuance must remain spendable once"
    );
    assert_eq!(balance.minted, 25);
    assert_eq!(
        (
            balance.reserved,
            balance.pending,
            balance.externalized,
            balance.burned
        ),
        (0, 0, 0, 0)
    );
    let original = original.expect("authorized issuance must retain a terminal outcome");
    assert_eq!(original.issuance, issuance.issuance);
    assert_eq!(retry, Some(original));
    assert_eq!(repeated_issuance, Some(original));
}
