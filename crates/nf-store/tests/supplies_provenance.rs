mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::*;
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use supplies_support::{Fixture, Scratch};
#[test]
fn three_trust_classes_and_distinct_lineages_remain_separate_after_replay_and_checkpoint() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let origins = [
        Origin {
            trust: TrustClass::Sandbox,
            lineage: [4; 32],
        },
        Origin {
            trust: TrustClass::CooperativeAudited,
            lineage: [4; 32],
        },
        Origin {
            trust: TrustClass::Canonical,
            lineage: [4; 32],
        },
        Origin {
            trust: TrustClass::Canonical,
            lineage: [5; 32],
        },
    ];
    f.policy.issuers.clear();
    f.policy.burners.clear();
    for origin in origins {
        f.policy.issuers.push(IssuerRule {
            issuer: f.issuer.account,
            content: SUPPLIES_CONTENT,
            origin,
            reason: IssuanceReason::AuthorityGrant,
            maximum: 1000,
        });
        f.policy.burners.push(BurnRule {
            issuer: f.issuer.account,
            content: SUPPLIES_CONTENT,
            origin,
            reason: BurnReason::AuthorityDestruction,
            maximum: 1000,
        });
    }
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    for (index, origin) in origins.into_iter().enumerate() {
        let mut value = f.issuance();
        value.request = RequestId::from_bytes([20 + index as u8; 16]);
        value.issuance = OperationId::from_bytes([40 + index as u8; 16]);
        value.origin = origin;
        value.amount = 20 + 10 * index as u64;
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&value));
        store.issue(&value, proof).unwrap().unwrap();
    }
    let base = f.issuance();
    let burn = Burn {
        request: RequestId::from_bytes([60; 16]),
        burn: OperationId::from_bytes([61; 16]),
        actor: base.actor,
        device: base.device,
        owner: base.beneficiary,
        universe: base.universe,
        history: base.history,
        policy: base.policy,
        content: base.content,
        origin: origins[2],
        reason: BurnReason::AuthorityDestruction,
        amount: 10,
    };
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&burn));
    store.burn(&burn, proof).unwrap().unwrap();
    let before = store.known_frontiers().unwrap();
    store.compact().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), before);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.policy, before).unwrap();
    for (index, origin) in origins.into_iter().enumerate() {
        let mut query = f.query();
        query.origin = origin;
        let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
        let value = store.balance(&query, proof).unwrap();
        let burned = if index == 2 { 10 } else { 0 };
        assert_eq!(
            (value.available, value.minted, value.burned),
            (
                20 + 10 * index as u64 - burned,
                20 + 10 * index as u64,
                burned
            )
        );
    }
    let mut absent = f.query();
    absent.origin.lineage = [99; 32];
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&absent));
    assert_eq!(store.balance(&absent, proof).unwrap(), Balances::default());
    assert_eq!(store.known_frontiers().unwrap(), before);
}
#[test]
fn unsupported_selected_policy_content_does_not_create_a_database() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    f.policy.issuers[0].content = [99; 32];
    assert!(matches!(
        SuppliesStore::create(scratch.db(), &f.policy, &f.membership),
        Err(Error::Rejected(SuppliesRejection::UnsupportedContent))
    ));
    assert!(!scratch.db().exists());
}
