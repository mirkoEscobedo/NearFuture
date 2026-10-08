mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{
    Burn, BurnReason, RejectedOutcome, RequestOutcome, RequestRejection, StatusQuery,
    SuppliesRejection,
};
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use supplies_support::{Fixture, Scratch};
#[test]
fn rejected_authority_burn_cannot_consume_later_funding_after_reopen_and_checkpoint() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let issuance = f.issuance();
    let burn = Burn {
        request: RequestId::from_bytes([35; 16]),
        burn: OperationId::from_bytes([36; 16]),
        actor: issuance.actor,
        device: issuance.device,
        owner: issuance.beneficiary,
        universe: issuance.universe,
        history: issuance.history,
        policy: issuance.policy,
        content: issuance.content,
        origin: issuance.origin,
        reason: BurnReason::AuthorityDestruction,
        amount: 8,
    };
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&burn));
    assert_eq!(
        store.burn(&burn, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    let rejected_known = store.known_frontiers().unwrap();
    assert_eq!(
        rejected_known.revision, 1,
        "first authority refusal is durable revision1"
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
        "grant follows the immutable refused burn"
    );
    let funded_known = store.known_frontiers().unwrap();
    assert_eq!(funded_known.revision, 2);
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&burn));
    let retry = store.burn(&burn, proof);
    assert_eq!(
        retry,
        Err(Error::Rejected(SuppliesRejection::Limit)),
        "a terminal authority refusal must not consume later funding"
    );
    let mut alias = burn;
    alias.request = RequestId::from_bytes([37; 16]);
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&alias));
    assert_eq!(
        store.burn(&alias, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), funded_known);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.policy, funded_known).unwrap();
    let mut later_alias = burn;
    later_alias.request = RequestId::from_bytes([38; 16]);
    for value in [burn, alias, later_alias] {
        let proof = f.attempt(&mut store, ChallengeRequest::Burn(&value));
        assert_eq!(
            store.burn(&value, proof),
            Err(Error::Rejected(SuppliesRejection::Limit))
        );
    }
    let own = f.query();
    let rejected = RequestOutcome::Rejected(RejectedOutcome {
        operation: burn.burn,
        revision: 1,
        reason: RequestRejection::InsufficientAvailable,
    });
    for request in [burn.request, alias.request, later_alias.request] {
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
            "stock owner sees the original refusal through all request aliases"
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
