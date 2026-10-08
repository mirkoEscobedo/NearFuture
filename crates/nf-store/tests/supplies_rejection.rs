mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{
    RejectedOutcome, RequestOutcome, RequestRejection, Reserve, StatusQuery, SuppliesRejection,
};
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use supplies_support::{Fixture, Scratch};
#[test]
fn rejected_reservation_cannot_spend_later_funding_after_reopen_and_checkpoint() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let own = f.query();
    let issuance = f.issuance();
    let reserve = Reserve {
        request: RequestId::from_bytes([31; 16]),
        reservation: OperationId::from_bytes([32; 16]),
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        policy: issuance.policy,
        content: own.content,
        origin: own.origin,
        amount: 8,
    };
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    assert_eq!(
        store.reserve(&reserve, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    let rejected_known = store.known_frontiers().unwrap();
    assert_eq!(
        rejected_known.revision, 1,
        "profile4 retains the first insufficient-stock decision"
    );
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.policy, rejected_known).unwrap();
    let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let grant = store
        .issue(&issuance, proof)
        .unwrap()
        .expect("real later authority grant25");
    assert_eq!(
        grant.revision, 2,
        "later funding follows the retained refusal"
    );
    let funded_known = store.known_frontiers().unwrap();
    assert_eq!(funded_known.revision, 2);
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    let retry = store.reserve(&reserve, proof);
    assert_eq!(
        retry,
        Err(Error::Rejected(SuppliesRejection::Limit)),
        "a terminal refusal must not become spendable after funding"
    );
    let mut alias = reserve;
    alias.request = RequestId::from_bytes([33; 16]);
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&alias));
    assert_eq!(
        store.reserve(&alias, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), funded_known);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.policy, funded_known).unwrap();
    let mut later_alias = reserve;
    later_alias.request = RequestId::from_bytes([34; 16]);
    for value in [reserve, alias, later_alias] {
        let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&value));
        assert_eq!(
            store.reserve(&value, proof),
            Err(Error::Rejected(SuppliesRejection::Limit))
        );
    }
    let rejected = RequestOutcome::Rejected(RejectedOutcome {
        operation: reserve.reservation,
        revision: 1,
        reason: RequestRejection::InsufficientAvailable,
    });
    for request in [reserve.request, alias.request, later_alias.request] {
        let query = StatusQuery {
            actor: own.actor,
            device: own.device,
            owner: own.owner,
            universe: own.universe,
            history: own.history,
            request,
        };
        let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
        assert_eq!(
            store.status(&query, proof).unwrap(),
            Some(rejected),
            "original and aliased requests retain the original refused operation and revision"
        );
    }
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&own));
    let balance = store.balance(&own, proof).unwrap();
    assert_eq!(
        (
            balance.available,
            balance.reserved,
            balance.pending,
            balance.externalized,
            balance.minted,
            balance.burned
        ),
        (25, 0, 0, 0, 25, 0)
    );
    assert_eq!(store.known_frontiers().unwrap(), funded_known);
}
