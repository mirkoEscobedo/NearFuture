use nf_contract::identity::{HistoryId, OperationId, RequestId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::{Invitation, MembershipState, PublicIdentity, Roles, Scope},
    persistence::redeem_persisted,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_kernel::{
    supplies::{
        BalanceQuery, Issuance, IssuanceReason, IssuerRule, Origin, SUPPLIES_CONTENT, StatusQuery,
        SuppliesPolicy, TrustClass, policy_digest,
    },
    trade::{
        AssetTerms, CancelOffer, CancelRule, OfferId, OfferTerms, ReserveOffer, ReservedOffer,
        TradeAdmission, TradeOutboxQuery, TradePolicy, TradeReceipt, TradeRequestOutcome,
    },
};
use nf_store::{
    supplies::ProofAttempt,
    trade::{TradeChallenge, TradeStore, TradeStoreError},
};
use std::path::PathBuf;

pub(super) struct Scratch(PathBuf);
impl Scratch {
    pub(super) fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let suffix = nf_identity::keys::random_id()
            .unwrap()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = root.join(format!("cancel-outbox-{}-{suffix}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(super) fn db(&self) -> PathBuf {
        self.0.join("supplies.sqlite")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        assert!(
            self.0
                .starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp"))
        );
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Member {
    identity: PublicIdentity,
    account: SecretSeed,
    device: SecretSeed,
}
pub(super) struct Keys {
    members: Vec<Member>,
    pub(super) policy: TradePolicy,
    scope: Scope,
    origin: Origin,
}
impl Keys {
    pub(super) fn new() -> Self {
        let members: Vec<_> = [b"outbox-A".as_slice(), b"outbox-B", b"outbox-C"]
            .into_iter()
            .map(|label| {
                let (identity, account, device) = generate_identity(label.to_vec()).unwrap();
                Member {
                    identity,
                    account,
                    device,
                }
            })
            .collect();
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
                issuer: members[0].identity.account,
                content: SUPPLIES_CONTENT,
                origin,
                reason: IssuanceReason::AuthorityGrant,
                maximum: 1000,
            }],
            burners: Vec::new(),
        };
        let policy = TradePolicy {
            supplies,
            clock_authority: members[0].identity.account,
            allowed_origins: vec![origin],
            admission: TradeAdmission::AuthorityVaultOnly,
            cancel_rule: CancelRule::EitherNamedParty,
        };
        Self {
            members,
            policy,
            scope,
            origin,
        }
    }
    pub(super) fn create(&self, path: PathBuf) -> TradeStore {
        let bootstrap = MembershipState::bootstrap(self.scope, &self.members[0].identity).unwrap();
        let mut store = TradeStore::create(path, &self.policy, &bootstrap).unwrap();
        for index in [1_usize, 2] {
            let player = &self.members[index];
            let invitation = Invitation {
                scope: self.scope,
                id: [index as u8; 16],
                issuer: self.members[0].identity.account,
                recipient: player.identity.clone(),
                roles: Roles::PLAYER,
                expires_at: 100,
                issued_revision: (index - 1) as u64,
                reusable: false,
            };
            let proof = admission_proof(&invitation, &player.account, &player.device).unwrap();
            let signed = sign_invitation(invitation, &self.members[0].account).unwrap();
            let admitted = redeem_persisted(&mut store, self.scope, &signed, &proof, 1).unwrap();
            assert_eq!(admitted.revision, index as u64);
            assert_eq!(
                admitted.accounts[&player.identity.account].roles,
                Roles::PLAYER
            );
        }
        assert_eq!(store.known_frontiers().unwrap().membership_revision, 2);
        store
    }
    pub(super) fn attempt(
        &self,
        store: &mut TradeStore,
        request: TradeChallenge<'_>,
    ) -> ProofAttempt {
        let issued = store.issue_challenge(request).unwrap();
        let mut proof = issued.template;
        let member = self
            .members
            .iter()
            .find(|member| member.identity.account == proof.account)
            .expect("the actor has a retained genuine device key");
        proof.signature = member.device.sign(&device_digest(&proof).unwrap());
        ProofAttempt {
            ticket: issued.ticket,
            proof,
        }
    }
    pub(super) fn grant(
        &self,
        store: &mut TradeStore,
        index: usize,
        amount: u64,
        request: u8,
        operation: u8,
        revision: u64,
    ) {
        let issue = Issuance {
            request: RequestId::from_bytes([request; 16]),
            issuance: OperationId::from_bytes([operation; 16]),
            actor: self.members[0].identity.account,
            device: self.members[0].identity.device,
            beneficiary: self.members[index].identity.account,
            universe: self.scope.universe,
            history: self.scope.history,
            policy: policy_digest(&self.policy.supplies).unwrap(),
            content: SUPPLIES_CONTENT,
            origin: self.origin,
            reason: IssuanceReason::AuthorityGrant,
            amount,
        };
        let proof = self.attempt(store, TradeChallenge::Issue(&issue));
        assert_eq!(
            store.issue(&issue, proof).unwrap().unwrap().revision,
            revision
        );
    }
    pub(super) fn reserve(
        &self,
        store: &mut TradeStore,
        taker: usize,
        ids: (u8, u8, u8),
        amounts: (u64, u64),
        revision: u64,
    ) -> (ReserveOffer, ReservedOffer) {
        let (offer, request, operation) = ids;
        let asset = |amount| AssetTerms {
            content: SUPPLIES_CONTENT,
            origin: self.origin,
            amount,
        };
        let value = ReserveOffer {
            request: RequestId::from_bytes([request; 16]),
            operation: OperationId::from_bytes([operation; 16]),
            maker_device: self.members[0].identity.device,
            taker_device: self.members[taker].identity.device,
            terms: OfferTerms {
                offer: OfferId::from_bytes([offer; 16]),
                version: 1,
                maker: self.members[0].identity.account,
                taker: self.members[taker].identity.account,
                universe: self.scope.universe,
                history: self.scope.history,
                policy: self.policy.digest().unwrap(),
                give: asset(amounts.0),
                want: asset(amounts.1),
                expires_at: 10,
            },
        };
        let maker = self.attempt(store, TradeChallenge::ReserveMaker(&value));
        let taker = self.attempt(store, TradeChallenge::ReserveTaker(&value));
        let reserved = store.reserve_offer(&value, maker, taker).unwrap();
        assert_eq!(reserved.revision, revision);
        (value, reserved)
    }
    pub(super) fn cancel(
        &self,
        store: &mut TradeStore,
        offer: ReservedOffer,
        request: u8,
        operation: u8,
        revision: u64,
    ) -> (CancelOffer, TradeReceipt) {
        let value = CancelOffer {
            request: RequestId::from_bytes([request; 16]),
            operation: OperationId::from_bytes([operation; 16]),
            actor: self.members[0].identity.account,
            device: self.members[0].identity.device,
            universe: self.scope.universe,
            history: self.scope.history,
            policy: self.policy.digest().unwrap(),
            offer: offer.offer,
            version: offer.version,
            digest: offer.digest,
        };
        let proof = self.attempt(store, TradeChallenge::Cancel(&value));
        let receipt = store.cancel_offer(&value, proof).unwrap();
        assert_eq!(receipt.revision, revision);
        (value, receipt)
    }
    pub(super) fn outbox(
        &self,
        store: &mut TradeStore,
        index: usize,
        cursor: u64,
        limit: u32,
    ) -> Result<Vec<TradeReceipt>, TradeStoreError> {
        let actor = &self.members[index].identity;
        let query = TradeOutboxQuery {
            actor: actor.account,
            device: actor.device,
            universe: self.scope.universe,
            history: self.scope.history,
            after_revision: cursor,
            limit,
        };
        let proof = self.attempt(store, TradeChallenge::Outbox(&query));
        store.outbox(&query, proof)
    }
    pub(super) fn balance(&self, store: &mut TradeStore, index: usize) -> [u64; 6] {
        let actor = &self.members[index].identity;
        let query = BalanceQuery {
            actor: actor.account,
            device: actor.device,
            owner: actor.account,
            universe: self.scope.universe,
            history: self.scope.history,
            content: SUPPLIES_CONTENT,
            origin: self.origin,
        };
        let proof = self.attempt(store, TradeChallenge::Balance(&query));
        let value = store.balance(&query, proof).unwrap();
        [
            value.available,
            value.reserved,
            value.pending,
            value.externalized,
            value.minted,
            value.burned,
        ]
    }
    pub(super) fn status(
        &self,
        store: &mut TradeStore,
        index: usize,
        request: RequestId,
    ) -> Option<TradeRequestOutcome> {
        let actor = &self.members[index].identity;
        let query = StatusQuery {
            actor: actor.account,
            device: actor.device,
            owner: actor.account,
            universe: self.scope.universe,
            history: self.scope.history,
            request,
        };
        let proof = self.attempt(store, TradeChallenge::Status(&query));
        store.status(&query, proof).unwrap()
    }
}
