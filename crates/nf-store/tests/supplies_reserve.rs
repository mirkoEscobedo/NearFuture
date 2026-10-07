mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{Reserve, policy_digest};
use nf_store::supplies::{ChallengeRequest, SuppliesStore};
use supplies_support::{Fixture, Scratch};
#[test]
fn reserve_stored_quantity_is_not_double_available_after_retry_and_checkpoint() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual grant25");
    let query = fixture.query();
    let reserve = Reserve {
        request: RequestId::from_bytes([14; 16]),
        reservation: OperationId::from_bytes([15; 16]),
        actor: query.actor,
        device: query.device,
        owner: query.owner,
        universe: query.universe,
        history: query.history,
        policy: policy_digest(&fixture.policy).unwrap(),
        content: query.content,
        origin: query.origin,
        amount: 8,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let original = store.reserve(&reserve, proof).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let retry = store.reserve(&reserve, proof).unwrap();
    let mut duplicate = reserve;
    duplicate.request = RequestId::from_bytes([16; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&duplicate));
    let repeated = store.reserve(&duplicate, proof).unwrap();
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&query));
    let balance = store.balance(&query, proof).unwrap();
    assert_eq!(
        balance.available, 17,
        "eight stored reserved units must stop being available once"
    );
    assert_eq!(
        (
            balance.reserved,
            balance.minted,
            balance.burned,
            balance.pending,
            balance.externalized
        ),
        (8, 25, 0, 0, 0)
    );
    let original = original.expect("actual persistent reservation outcome");
    assert_eq!(
        (original.reservation, original.revision),
        (reserve.reservation, 2)
    );
    assert_eq!(retry, Some(original));
    assert_eq!(repeated, Some(original));
}
