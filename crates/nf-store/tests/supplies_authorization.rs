mod supplies_support;
use nf_contract::identity::*;
use nf_identity::{
    keys::generate_identity, model::*, rotation::revocation_digest, signing::device_digest,
};
use nf_kernel::supplies::*;
use nf_store::supplies::{
    ChallengeRequest, ProofAttempt, SuppliesStore, SuppliesStoreError as Error,
};
use supplies_support::{Fixture, Scratch};
#[test]
fn player_and_worker_cannot_mint_or_burn_even_with_selected_rules() {
    for role in [Roles::PLAYER, Roles::WORKER] {
        let scratch = Scratch::new();
        let mut f = Fixture::new_with_roles(role);
        let mut rule = f.policy.issuers[0];
        rule.issuer = f.beneficiary.account;
        f.policy.issuers.push(rule);
        let mut rule = f.policy.burners[0];
        rule.issuer = f.beneficiary.account;
        f.policy.burners.push(rule);
        let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
        let mut grant = f.issuance();
        grant.beneficiary = f.issuer.account;
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&grant));
        store.issue(&grant, proof).unwrap().unwrap();
        let before = store.known_frontiers().unwrap();
        let mut attempt = grant;
        attempt.request = RequestId::from_bytes([80; 16]);
        attempt.issuance = OperationId::from_bytes([81; 16]);
        attempt.actor = f.beneficiary.account;
        attempt.device = f.beneficiary.device;
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&attempt));
        assert_eq!(
            store.issue(&attempt, proof),
            Err(Error::Identity(IdentityError::RolePolicy))
        );
        let burn = Burn {
            request: RequestId::from_bytes([82; 16]),
            burn: OperationId::from_bytes([83; 16]),
            actor: attempt.actor,
            device: attempt.device,
            owner: grant.beneficiary,
            universe: grant.universe,
            history: grant.history,
            policy: grant.policy,
            content: grant.content,
            origin: grant.origin,
            reason: BurnReason::AuthorityDestruction,
            amount: 10,
        };
        let proof = f.attempt(&mut store, ChallengeRequest::Burn(&burn));
        assert_eq!(
            store.burn(&burn, proof),
            Err(Error::Identity(IdentityError::RolePolicy))
        );
        assert_eq!(store.known_frontiers().unwrap(), before);
        let mut query = f.query();
        query.actor = f.issuer.account;
        query.device = f.issuer.device;
        query.owner = f.issuer.account;
        let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
        assert_eq!(store.balance(&query, proof).unwrap().available, 25);
    }
}
#[test]
fn bad_signature_and_reused_real_ticket_do_not_issue_assets() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let issuance = f.issuance();
    let mut proof = f.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let valid = ProofAttempt {
        ticket: proof.ticket,
        proof: proof.proof.clone(),
    };
    proof.proof.signature[0] ^= 1;
    assert_eq!(
        store.issue(&issuance, proof),
        Err(Error::Identity(IdentityError::Signature))
    );
    assert_eq!(store.issue(&issuance, valid), Err(Error::Replay));
    let query = f.query();
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(store.balance(&query, proof).unwrap().available, 0);
    let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issuance));
    let repeated = ProofAttempt {
        ticket: proof.ticket,
        proof: proof.proof.clone(),
    };
    let original = store.issue(&issuance, proof).unwrap().unwrap();
    assert_eq!(store.issue(&issuance, repeated), Err(Error::Replay));
    assert_eq!(original.revision, 1);
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(store.balance(&query, proof).unwrap().available, 25);
}
#[test]
fn cross_history_unknown_content_and_other_accounts_reject_without_materialization() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let original = f.issuance();
    let proof = f.attempt(&mut store, ChallengeRequest::Issue(&original));
    store.issue(&original, proof).unwrap().unwrap();
    let before = store.known_frontiers().unwrap();
    for (index, reason) in [
        SuppliesRejection::Scope,
        SuppliesRejection::UnsupportedContent,
        SuppliesRejection::Unauthorized,
    ]
    .into_iter()
    .enumerate()
    {
        let mut value = original;
        value.request = RequestId::from_bytes([90 + index as u8; 16]);
        value.issuance = OperationId::from_bytes([94 + index as u8; 16]);
        match index {
            0 => value.history = HistoryId::from_bytes([99; 16]),
            1 => value.content = [99; 32],
            _ => value.beneficiary = AccountId::from_bytes([99; 16]),
        }
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&value));
        assert_eq!(store.issue(&value, proof), Err(Error::Rejected(reason)));
    }
    let query = f.query();
    for (index, reason) in [
        SuppliesRejection::Scope,
        SuppliesRejection::UnsupportedContent,
        SuppliesRejection::Unauthorized,
    ]
    .into_iter()
    .enumerate()
    {
        let mut value = query;
        match index {
            0 => value.history = HistoryId::from_bytes([99; 16]),
            1 => value.content = [99; 32],
            _ => value.owner = f.issuer.account,
        }
        let proof = f.attempt(&mut store, ChallengeRequest::Balance(&value));
        assert_eq!(store.balance(&value, proof), Err(Error::Rejected(reason)));
    }
    assert_eq!(store.known_frontiers().unwrap(), before);
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&query));
    assert_eq!(store.balance(&query, proof).unwrap().available, 25);
}
#[test]
fn signed_persisted_revocation_fences_and_consumes_preissued_proof() {
    let scratch = Scratch::new();
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let (owner, account, key) = generate_identity(b"supplies-revocation-owner".to_vec()).unwrap();
    let membership = MembershipState::bootstrap(scope, &owner).unwrap();
    let origin = Origin {
        trust: TrustClass::Canonical,
        lineage: [4; 32],
    };
    let policy = SuppliesPolicy {
        universe: scope.universe,
        history: scope.history,
        ruleset: [3; 32],
        issuers: vec![IssuerRule {
            issuer: owner.account,
            content: SUPPLIES_CONTENT,
            origin,
            reason: IssuanceReason::AuthorityGrant,
            maximum: 1000,
        }],
        burners: vec![],
    };
    let mut store = SuppliesStore::create(scratch.db(), &policy, &membership).unwrap();
    let issuance = Issuance {
        request: RequestId::from_bytes([8; 16]),
        issuance: OperationId::from_bytes([9; 16]),
        actor: owner.account,
        device: owner.device,
        beneficiary: owner.account,
        universe: scope.universe,
        history: scope.history,
        policy: policy_digest(&policy).unwrap(),
        content: SUPPLIES_CONTENT,
        origin,
        reason: IssuanceReason::AuthorityGrant,
        amount: 25,
    };
    let issued = store
        .issue_challenge(ChallengeRequest::Issue(&issuance))
        .unwrap();
    let mut signed = issued.template;
    signed.signature = key.sign(&device_digest(&signed).unwrap());
    let proof = ProofAttempt {
        ticket: issued.ticket,
        proof: signed.clone(),
    };
    let repeated = ProofAttempt {
        ticket: issued.ticket,
        proof: signed,
    };
    let change = DeviceRevocation {
        scope,
        issuer: owner.account,
        device: owner.device,
        frontier: membership.revision,
    };
    let next = membership
        .revoke_device(&change, &account.sign(&revocation_digest(&change)))
        .unwrap();
    store
        .commit_membership(Some(membership.revision), &next)
        .unwrap();
    assert_eq!(
        store.issue(&issuance, proof),
        Err(Error::Identity(IdentityError::Frontier))
    );
    assert_eq!(store.issue(&issuance, repeated), Err(Error::Replay));
    assert!(matches!(
        store.issue_challenge(ChallengeRequest::Issue(&issuance)),
        Err(Error::Identity(IdentityError::Revoked))
    ));
    let known = store.known_frontiers().unwrap();
    assert_eq!(
        (known.revision, known.membership_revision),
        (0, next.revision)
    );
    drop(store);
    let reopened = SuppliesStore::open_existing(scratch.db(), &policy, known).unwrap();
    assert_eq!(reopened.known_frontiers().unwrap(), known);
}
