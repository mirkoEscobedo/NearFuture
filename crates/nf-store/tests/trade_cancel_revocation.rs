use nf_contract::identity::{HistoryId, OperationId, RequestId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::{
        DeviceRevocation, IdentityError, Invitation, MembershipState, PublicIdentity, Roles, Scope,
    },
    persistence::{redeem_persisted, revoke_persisted},
    rotation::revocation_digest,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_kernel::{
    supplies::{
        BalanceQuery, Balances, Issuance, IssuanceReason, IssuerRule, Origin, SUPPLIES_CONTENT,
        StatusQuery, SuppliesPolicy, TrustClass, policy_digest,
    },
    trade::{
        AssetTerms, CancelOffer, CancelRule, OfferId, OfferState, OfferStatusQuery, OfferTerms,
        ReserveOffer, TradeAdmission, TradeOutboxQuery, TradePolicy, TradeRequestOutcome,
    },
};
use nf_store::{
    supplies::{IssuedChallenge, ProofAttempt, SuppliesStoreError},
    trade::{TradeChallenge, TradeStore, TradeStoreError as Error},
};
use std::path::PathBuf;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let suffix = nf_identity::keys::random_id()
            .unwrap()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = root.join(format!("cancel-revocation-{}-{suffix}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn db(&self) -> PathBuf {
        self.0.join("supplies.sqlite")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        assert!(self.0.starts_with(root));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Members {
    owner: PublicIdentity,
    maker: PublicIdentity,
    owner_account: SecretSeed,
    maker_account: SecretSeed,
    owner_device: SecretSeed,
    maker_device: SecretSeed,
}
impl Members {
    fn new() -> Self {
        let (owner, owner_account, owner_device) =
            generate_identity(b"cancel-owner".to_vec()).unwrap();
        let (maker, maker_account, maker_device) =
            generate_identity(b"cancel-player".to_vec()).unwrap();
        Self {
            owner,
            maker,
            owner_account,
            maker_account,
            owner_device,
            maker_device,
        }
    }
    fn attempt(&self, store: &mut TradeStore, request: TradeChallenge<'_>) -> ProofAttempt {
        self.sign(store.issue_challenge(request).unwrap())
    }
    fn sign(&self, issued: IssuedChallenge) -> ProofAttempt {
        let mut proof = issued.template;
        let key = if proof.account == self.owner.account {
            &self.owner_device
        } else {
            assert_eq!(proof.account, self.maker.account);
            &self.maker_device
        };
        proof.signature = key.sign(&device_digest(&proof).unwrap());
        ProofAttempt {
            ticket: issued.ticket,
            proof,
        }
    }
}

#[test]
fn signed_self_revocation_invalidates_live_cancel_proof_without_releasing_joint_escrow() {
    let scratch = Scratch::new();
    let keys = Members::new();
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let origin = Origin {
        trust: TrustClass::Canonical,
        lineage: [4; 32],
    };
    let supplies = SuppliesPolicy {
        universe: scope.universe,
        history: scope.history,
        ruleset: [3; 32],
        issuers: vec![IssuerRule {
            issuer: keys.owner.account,
            content: SUPPLIES_CONTENT,
            origin,
            reason: IssuanceReason::AuthorityGrant,
            maximum: 1000,
        }],
        burners: Vec::new(),
    };
    let policy = TradePolicy {
        supplies,
        clock_authority: keys.owner.account,
        allowed_origins: vec![origin],
        admission: TradeAdmission::AuthorityVaultOnly,
        cancel_rule: CancelRule::EitherNamedParty,
    };
    let bootstrap = MembershipState::bootstrap(scope, &keys.owner).unwrap();
    assert_eq!(bootstrap.revision, 0);
    let mut store = TradeStore::create(scratch.db(), &policy, &bootstrap).unwrap();
    let invitation = Invitation {
        scope,
        id: [11; 16],
        issuer: keys.owner.account,
        recipient: keys.maker.clone(),
        roles: Roles::PLAYER,
        expires_at: 100,
        issued_revision: 0,
        reusable: false,
    };
    let admission = admission_proof(&invitation, &keys.maker_account, &keys.maker_device).unwrap();
    let signed = sign_invitation(invitation, &keys.owner_account).unwrap();
    let admitted = redeem_persisted(&mut store, scope, &signed, &admission, 1).unwrap();
    assert_eq!(admitted.revision, 1);
    assert_eq!(admitted.accounts[&keys.maker.account].roles, Roles::PLAYER);
    for (beneficiary, amount, request, operation, revision) in [
        (keys.maker.account, 25, 80, 81, 1),
        (keys.owner.account, 10, 82, 83, 2),
    ] {
        let issue = Issuance {
            request: RequestId::from_bytes([request; 16]),
            issuance: OperationId::from_bytes([operation; 16]),
            actor: keys.owner.account,
            device: keys.owner.device,
            beneficiary,
            universe: scope.universe,
            history: scope.history,
            policy: policy_digest(&policy.supplies).unwrap(),
            content: SUPPLIES_CONTENT,
            origin,
            reason: IssuanceReason::AuthorityGrant,
            amount,
        };
        let proof = keys.attempt(&mut store, TradeChallenge::Issue(&issue));
        assert_eq!(
            store.issue(&issue, proof).unwrap().unwrap().revision,
            revision
        );
    }
    let reserve = ReserveOffer {
        request: RequestId::from_bytes([84; 16]),
        operation: OperationId::from_bytes([85; 16]),
        maker_device: keys.maker.device,
        taker_device: keys.owner.device,
        terms: OfferTerms {
            offer: OfferId::from_bytes([90; 16]),
            version: 1,
            maker: keys.maker.account,
            taker: keys.owner.account,
            universe: scope.universe,
            history: scope.history,
            policy: policy.digest().unwrap(),
            give: AssetTerms {
                content: SUPPLIES_CONTENT,
                origin,
                amount: 8,
            },
            want: AssetTerms {
                content: SUPPLIES_CONTENT,
                origin,
                amount: 5,
            },
            expires_at: 10,
        },
    };
    let maker = keys.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
    let taker = keys.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
    let reserved = store.reserve_offer(&reserve, maker, taker).unwrap();
    assert_eq!(reserved.revision, 3);
    let maker_balance = BalanceQuery {
        actor: keys.maker.account,
        device: keys.maker.device,
        owner: keys.maker.account,
        universe: scope.universe,
        history: scope.history,
        content: SUPPLIES_CONTENT,
        origin,
    };
    let proof = keys.attempt(&mut store, TradeChallenge::Balance(&maker_balance));
    assert_eq!(
        store.balance(&maker_balance, proof).unwrap(),
        Balances {
            available: 17,
            reserved: 8,
            pending: 0,
            externalized: 0,
            minted: 25,
            burned: 0,
        }
    );
    let before = store.known_frontiers().unwrap();
    assert_eq!((before.revision, before.membership_revision), (3, 1));
    let cancel = CancelOffer {
        request: RequestId::from_bytes([92; 16]),
        operation: OperationId::from_bytes([91; 16]),
        actor: keys.maker.account,
        device: keys.maker.device,
        universe: scope.universe,
        history: scope.history,
        policy: policy.digest().unwrap(),
        offer: reserved.offer,
        version: 1,
        digest: reserved.digest,
    };
    // The genuine live proof is issued/signed at member1 before the genuine signed self-revocation.
    let old = keys.attempt(&mut store, TradeChallenge::Cancel(&cancel));
    let reused = ProofAttempt {
        ticket: old.ticket,
        proof: old.proof.clone(),
    };
    let change = DeviceRevocation {
        scope,
        issuer: keys.maker.account,
        device: keys.maker.device,
        frontier: 1,
    };
    let signature = keys.maker_account.sign(&revocation_digest(&change));
    let revoked = revoke_persisted(&mut store, scope, &change, &signature).unwrap();
    assert_eq!(revoked.revision, 2);
    assert!(revoked.devices[&keys.maker.device].revoked);
    assert_eq!(
        store.cancel_offer(&cancel, old),
        Err(Error::Supplies(SuppliesStoreError::Identity(
            IdentityError::Frontier
        )))
    );
    assert_eq!(
        store.cancel_offer(&cancel, reused),
        Err(Error::Supplies(SuppliesStoreError::Replay))
    );
    assert_eq!(
        store.issue_challenge(TradeChallenge::Cancel(&cancel)).err(),
        Some(Error::Supplies(SuppliesStoreError::Identity(
            IdentityError::Revoked
        )))
    );
    let retained = store.known_frontiers().unwrap();
    assert_eq!((retained.revision, retained.membership_revision), (3, 2));
    assert_eq!((retained.clock_tick, retained.clock_revision), (0, 0));
    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = TradeStore::open_existing(scratch.db(), &policy, retained).unwrap();
        }
        // Only the unrevoked taker can issue these fresh current-membership query proofs.
        let mut status = StatusQuery {
            actor: keys.owner.account,
            device: keys.owner.device,
            owner: keys.owner.account,
            universe: scope.universe,
            history: scope.history,
            request: cancel.request,
        };
        let proof = keys.attempt(&mut store, TradeChallenge::Status(&status));
        assert_eq!(store.status(&status, proof).unwrap(), None);
        status.request = reserve.request;
        let proof = keys.attempt(&mut store, TradeChallenge::Status(&status));
        assert_eq!(
            store.status(&status, proof).unwrap(),
            Some(TradeRequestOutcome::Reserved {
                operation: OperationId::from_bytes([85; 16]),
                offer: reserved
            })
        );
        let query = OfferStatusQuery {
            actor: keys.owner.account,
            device: keys.owner.device,
            universe: scope.universe,
            history: scope.history,
            offer: reserved.offer,
            version: 1,
        };
        let proof = keys.attempt(&mut store, TradeChallenge::OfferStatus(&query));
        assert_eq!(
            store.offer_state(&query, proof).unwrap(),
            Some(OfferState::Reserved(reserved))
        );
        let query = BalanceQuery {
            actor: keys.owner.account,
            device: keys.owner.device,
            owner: keys.owner.account,
            universe: scope.universe,
            history: scope.history,
            content: SUPPLIES_CONTENT,
            origin,
        };
        let proof = keys.attempt(&mut store, TradeChallenge::Balance(&query));
        assert_eq!(
            store.balance(&query, proof).unwrap(),
            Balances {
                available: 5,
                reserved: 5,
                pending: 0,
                externalized: 0,
                minted: 10,
                burned: 0,
            }
        );
        let query = TradeOutboxQuery {
            actor: keys.owner.account,
            device: keys.owner.device,
            universe: scope.universe,
            history: scope.history,
            after_revision: 0,
            limit: 16,
        };
        let proof = keys.attempt(&mut store, TradeChallenge::Outbox(&query));
        assert!(store.outbox(&query, proof).unwrap().is_empty());
        assert_eq!(
            store.issue_challenge(TradeChallenge::Cancel(&cancel)).err(),
            Some(Error::Supplies(SuppliesStoreError::Identity(
                IdentityError::Revoked
            )))
        );
        assert_eq!(store.known_frontiers().unwrap(), retained);
    }
}
