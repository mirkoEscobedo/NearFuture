mod supplies_support;
use nf_store::supplies::{ChallengeRequest, SuppliesStore};
use supplies_support::{Fixture, Scratch};

#[test]
fn compact_checkpoint_preserves_issuance_request_outcome_and_revision() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let original = store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual retained issuance");
    let query = fixture.query();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(store.balance(&query, proof).unwrap().available, 25);
    let before = store.known_frontiers().unwrap();
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), before);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    assert_eq!(store.issue(&issuance, proof).unwrap(), Some(original));
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, before).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&query));
    let balance = store.balance(&query, proof).unwrap();
    assert_eq!(
        (balance.available, balance.minted, balance.burned),
        (25, 25, 0)
    );
}
