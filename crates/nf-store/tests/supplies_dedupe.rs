mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::*;
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use supplies_support::{Fixture, Scratch};
fn burn(v: Issuance) -> Burn {
    Burn {
        request: RequestId::from_bytes([60; 16]),
        burn: OperationId::from_bytes([61; 16]),
        actor: v.actor,
        device: v.device,
        owner: v.beneficiary,
        universe: v.universe,
        history: v.history,
        policy: v.policy,
        content: v.content,
        origin: v.origin,
        reason: BurnReason::AuthorityDestruction,
        amount: 10,
    }
}
#[test]
fn operation_and_request_ids_are_global_across_issuance_and_burn() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let issuance = f.issuance();
    let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    store.issue(&issuance, proof).unwrap().unwrap();
    let original = burn(issuance);
    for by_request in [false, true] {
        let mut attempt = original;
        if by_request {
            attempt.request = issuance.request
        } else {
            attempt.burn = issuance.issuance
        }
        let proof = f.attempt(&mut store, ChallengeRequest::Burn(&attempt));
        assert_eq!(
            store.burn(&attempt, proof),
            Err(Error::Rejected(SuppliesRejection::Conflict))
        );
    }
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&original));
    store.burn(&original, proof).unwrap().unwrap();
    let before = store.known_frontiers().unwrap();
    for by_request in [false, true] {
        let mut attempt = issuance;
        attempt.request = RequestId::from_bytes([62; 16]);
        attempt.issuance = OperationId::from_bytes([63; 16]);
        if by_request {
            attempt.request = original.request
        } else {
            attempt.issuance = original.burn
        }
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&attempt));
        assert_eq!(
            store.issue(&attempt, proof),
            Err(Error::Rejected(SuppliesRejection::Conflict))
        );
    }
    assert_eq!(store.known_frontiers().unwrap(), before);
    store.compact().unwrap();
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.policy, before).unwrap();
    let query = f.query();
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(
        (
            store.balance(&query, proof).unwrap().available,
            before.revision
        ),
        (15, 2)
    );
}
#[test]
fn same_operation_with_changed_owner_amount_or_allowed_origin_conflicts_without_new_effect() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let origin = Origin {
        trust: TrustClass::Canonical,
        lineage: [5; 32],
    };
    let mut rule = f.policy.issuers[0];
    rule.origin = origin;
    f.policy.issuers.push(rule);
    let mut rule = f.policy.burners[0];
    rule.origin = origin;
    f.policy.burners.push(rule);
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let issuance = f.issuance();
    let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let issued = store.issue(&issuance, proof).unwrap().unwrap();
    for index in 0..3 {
        let mut changed = issuance;
        changed.request = RequestId::from_bytes([70 + index; 16]);
        match index {
            0 => changed.amount += 1,
            1 => changed.beneficiary = f.issuer.account,
            _ => changed.origin = origin,
        }
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&changed));
        assert_eq!(
            store.issue(&changed, proof),
            Err(Error::Rejected(SuppliesRejection::Conflict))
        );
    }
    let original = burn(issuance);
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&original));
    let burned = store.burn(&original, proof).unwrap().unwrap();
    let before = store.known_frontiers().unwrap();
    for index in 0..3 {
        let mut changed = original;
        changed.request = RequestId::from_bytes([74 + index; 16]);
        match index {
            0 => changed.amount -= 1,
            1 => changed.owner = f.issuer.account,
            _ => changed.origin = origin,
        }
        let proof = f.attempt(&mut store, ChallengeRequest::Burn(&changed));
        assert_eq!(
            store.burn(&changed, proof),
            Err(Error::Rejected(SuppliesRejection::Conflict))
        );
    }
    assert_eq!(store.known_frontiers().unwrap(), before);
    let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    assert_eq!(store.issue(&issuance, proof).unwrap(), Some(issued));
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&original));
    assert_eq!(store.burn(&original, proof).unwrap(), Some(burned));
    let query = f.query();
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(store.balance(&query, proof).unwrap().available, 15);
}
