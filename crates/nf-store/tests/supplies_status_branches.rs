mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_identity::model::Roles;
use nf_kernel::supplies::{
    Balances, Burn, BurnReason, RequestOutcome, StatusQuery, SuppliesRejection, policy_digest,
};
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use supplies_support::{Fixture, Scratch};

#[test]
fn signed_stock_owner_status_recovers_original_burn_after_restart_and_checkpoint() {
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
    let burn = Burn {
        request: RequestId::from_bytes([71; 16]),
        burn: OperationId::from_bytes([72; 16]),
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
    let original = store
        .burn(&burn, proof)
        .unwrap()
        .expect("actual signed durable burn");
    assert_eq!((original.burn, original.revision), (burn.burn, 2));
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let own = fixture.query();
    let query = StatusQuery {
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        request: burn.request,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Status(&query));
    assert_eq!(
        store.status(&query, proof).unwrap(),
        Some(RequestOutcome::Burned(original))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
}

#[test]
fn signed_stock_owner_status_for_unknown_request_returns_none_without_history_change() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issuance = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual signed known request");
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
        request: RequestId::from_bytes([73; 16]),
    };
    assert_ne!(query.request, issuance.request);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Status(&query));
    assert_eq!(store.status(&query, proof).unwrap(), None);
    assert_eq!(store.known_frontiers().unwrap(), known);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    assert_eq!(store.balance(&own, proof).unwrap().available, 25);
    assert_eq!(store.known_frontiers().unwrap(), known);
}

#[test]
fn authorized_player_status_cannot_read_a_known_foreign_stock_request() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    assert_ne!(fixture.issuer.account, fixture.beneficiary.account);
    assert_eq!(
        fixture
            .membership
            .accounts
            .get(&fixture.beneficiary.account)
            .unwrap()
            .roles,
        Roles::PLAYER
    );
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    // The real authority grants to its own stock; the separately admitted PLAYER owns no units.
    let mut issuance = fixture.issuance();
    issuance.beneficiary = fixture.issuer.account;
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store
        .issue(&issuance, proof)
        .unwrap()
        .expect("actual retained foreign stock request");
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 1);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let own = fixture.query();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    assert_eq!(
        store.balance(&own, proof).unwrap(),
        Balances::default(),
        "real PLAYER own-stock authorization succeeds"
    );
    let query = StatusQuery {
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        request: issuance.request,
    };
    assert_eq!(query.actor, query.owner);
    assert_ne!(query.owner, issuance.beneficiary);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Status(&query));
    assert_eq!(
        store.status(&query, proof),
        Err(Error::Rejected(SuppliesRejection::Unauthorized))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
}
