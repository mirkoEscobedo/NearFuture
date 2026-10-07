use super::protocol::{self, Cut, Descriptor};
use nf_contract::identity::{HistoryId, OperationId, RequestId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::{MembershipState, PublicIdentity, Scope},
    signing::device_digest,
};
use nf_kernel::supplies::*;
use nf_store::supplies::{ChallengeRequest, KnownSuppliesFrontiers, ProofAttempt, SuppliesStore};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Fixture {
    root: PathBuf,
    base: PathBuf,
    pub identity: PublicIdentity,
    _account: SecretSeed,
    key: SecretSeed,
    pub issue: Issuance,
    pub policy: SuppliesPolicy,
    membership: MembershipState,
}
impl Fixture {
    pub fn new(history: u8) -> Self {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&base).unwrap_or_else(|_| panic!("owned worker scratch parent"));
        protocol::directory(&base);
        let base = base
            .canonicalize()
            .unwrap_or_else(|_| panic!("absolute worker scratch parent"));
        let root = base.join(format!(
            "supplies-worker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap_or_else(|_| panic!("fresh owned worker scratch"));
        protocol::directory(&root);
        let (identity, account, key) =
            generate_identity(b"supplies-public-worker".to_vec()).unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([history; 16]),
        };
        let membership = MembershipState::bootstrap(scope, &identity).unwrap();
        let mut issue = Issuance {
            request: RequestId::from_bytes([8; 16]),
            issuance: OperationId::from_bytes([9; 16]),
            actor: identity.account,
            device: identity.device,
            beneficiary: identity.account,
            universe: scope.universe,
            history: scope.history,
            policy: [0; 32],
            content: SUPPLIES_CONTENT,
            origin: Origin {
                trust: TrustClass::Canonical,
                lineage: [4; 32],
            },
            reason: IssuanceReason::AuthorityGrant,
            amount: 25,
        };
        let policy = protocol::policy(&issue);
        issue.policy = policy_digest(&policy).unwrap();
        Self {
            root,
            base,
            identity,
            _account: account,
            key,
            issue,
            policy,
            membership,
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn create(&self) -> SuppliesStore {
        SuppliesStore::create(
            self.root.join("supplies.sqlite"),
            &self.policy,
            &self.membership,
        )
        .unwrap()
    }
    pub fn reopen(&self, known: KnownSuppliesFrontiers) -> SuppliesStore {
        SuppliesStore::open_existing(self.root.join("supplies.sqlite"), &self.policy, known)
            .unwrap()
    }
    pub fn descriptor(&self, cut: Cut, known: KnownSuppliesFrontiers) {
        Descriptor {
            cut,
            issue: self.issue,
            known,
            peer: self.identity.peer.clone(),
        }
        .write(&self.root);
    }
    pub fn attempt(
        &self,
        store: &mut SuppliesStore,
        request: ChallengeRequest<'_>,
    ) -> ProofAttempt {
        let issued = store.issue_challenge(request).unwrap();
        let mut proof = issued.template;
        proof.signature = self.key.sign(&device_digest(&proof).unwrap());
        ProofAttempt {
            ticket: issued.ticket,
            proof,
        }
    }
    pub fn sign_actual_template(
        &self,
        bytes: &[u8; 297],
        known: KnownSuppliesFrontiers,
    ) -> [u8; 64] {
        let proof = protocol::decode_template(bytes);
        assert_eq!(proof.scope, self.membership.scope);
        assert_eq!(proof.account, self.identity.account);
        assert_eq!(proof.device, self.identity.device);
        assert_eq!(proof.frontier, known.membership_revision);
        assert_eq!(proof.peer, self.identity.peer);
        assert_eq!(proof.signature, [0; 64]);
        self.key.sign(&device_digest(&proof).unwrap())
    }
    pub fn observe(
        &self,
        store: &mut SuppliesStore,
        expected_units: u64,
        revision: u64,
        requests: &[RequestId],
    ) {
        let query = BalanceQuery {
            actor: self.identity.account,
            device: self.identity.device,
            owner: self.identity.account,
            universe: self.policy.universe,
            history: self.policy.history,
            content: SUPPLIES_CONTENT,
            origin: self.issue.origin,
        };
        let proof = self.attempt(store, ChallengeRequest::Balance(&query));
        let balance = store.balance(&query, proof).unwrap();
        // Literal independent six-bucket oracle; no product transition/replay supplies expected quantities.
        assert_eq!(
            (
                balance.available,
                balance.reserved,
                balance.pending,
                balance.externalized,
                balance.minted,
                balance.burned
            ),
            (expected_units, 0, 0, 0, expected_units, 0)
        );
        let known = store.known_frontiers().unwrap();
        assert_eq!(
            (known.scope, known.revision, known.membership_revision),
            (self.membership.scope, revision, 0)
        );
        for &request in requests {
            let query = StatusQuery {
                actor: self.identity.account,
                device: self.identity.device,
                owner: self.identity.account,
                universe: self.policy.universe,
                history: self.policy.history,
                request,
            };
            let proof = self.attempt(store, ChallengeRequest::Status(&query));
            let expected = if revision == 0 {
                None
            } else {
                Some(RequestOutcome::Issued(IssuanceOutcome {
                    issuance: self.issue.issuance,
                    revision: 1,
                }))
            };
            assert_eq!(store.status(&query, proof).unwrap(), expected);
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        protocol::directory(&self.base);
        protocol::directory(&self.root);
        assert_eq!(
            self.root.parent(),
            Some(self.base.as_path()),
            "exact absolute test-owned cleanup target"
        );
        std::fs::remove_dir_all(&self.root)
            .unwrap_or_else(|_| panic!("owned worker scratch cleanup"));
    }
}
