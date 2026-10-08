mod supplies_support;
use nf_kernel::supplies::{RequestOutcome, StatusQuery};
use nf_store::supplies::{ChallengeRequest, SuppliesStore};
use supplies_support::{Fixture, Scratch};
#[test]
fn signed_owner_status_recovers_original_request_outcome_after_checkpoint() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let original = store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual retained grant");
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let own = fixture.query();
    let query = StatusQuery {
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        request: issuance.request,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Status(&query));
    let outcome = store.status(&query, proof).unwrap();
    assert_eq!(
        outcome,
        Some(RequestOutcome::Issued(original)),
        "signed stock owner must recover the durable original outcome"
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
}
