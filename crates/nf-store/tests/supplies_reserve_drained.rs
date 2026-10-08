mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{Reserve, policy_digest};
use nf_store::supplies::{ChallengeRequest, SuppliesStore};
use supplies_support::{Fixture, Scratch};

#[test]
fn full_balance_reservation_retries_after_reopen_and_checkpoint_with_zero_available() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual signed grant25");
    let query = fixture.query();
    let reserve = Reserve {
        request: RequestId::from_bytes([31; 16]),
        reservation: OperationId::from_bytes([32; 16]),
        actor: query.actor,
        device: query.device,
        owner: query.owner,
        universe: query.universe,
        history: query.history,
        policy: policy_digest(&fixture.policy).unwrap(),
        content: query.content,
        origin: query.origin,
        amount: 25,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let original = store.reserve(&reserve, proof).unwrap();
    let original_known = store.known_frontiers().unwrap();
    drop(store);

    let mut store =
        SuppliesStore::open_existing(scratch.db(), &fixture.policy, original_known).unwrap();
    // Capture Results so a wrong affordability-first retry cannot shadow the economic oracle.
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let exact = store.reserve(&reserve, proof);
    let mut alias = reserve;
    alias.request = RequestId::from_bytes([33; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&alias));
    let fresh_request = store.reserve(&alias, proof);
    store.compact().unwrap();
    let checkpoint_known = store.known_frontiers().unwrap();
    drop(store);

    let mut store =
        SuppliesStore::open_existing(scratch.db(), &fixture.policy, checkpoint_known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let exact_after_checkpoint = store.reserve(&reserve, proof);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&alias));
    let persisted_alias = store.reserve(&alias, proof);
    let mut later_alias = reserve;
    later_alias.request = RequestId::from_bytes([34; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&later_alias));
    let fresh_after_checkpoint = store.reserve(&later_alias, proof);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&query));
    let balance = store.balance(&query, proof).unwrap();
    assert_eq!(
        balance.available, 0,
        "all25 reserved units must stop being available before any None unwrap"
    );
    assert_eq!(
        (
            balance.reserved,
            balance.minted,
            balance.burned,
            balance.pending,
            balance.externalized
        ),
        (25, 25, 0, 0, 0)
    );
    let original = original.expect("actual immutable full-balance reservation outcome");
    assert_eq!(
        (original.reservation, original.revision),
        (reserve.reservation, 2)
    );
    for retry in [
        exact,
        fresh_request,
        exact_after_checkpoint,
        persisted_alias,
        fresh_after_checkpoint,
    ] {
        assert_eq!(
            retry.expect("signed retry must bypass affordability for original operation"),
            Some(original)
        );
    }
    assert_eq!(checkpoint_known, original_known);
    assert_eq!(store.known_frontiers().unwrap(), original_known);
}
