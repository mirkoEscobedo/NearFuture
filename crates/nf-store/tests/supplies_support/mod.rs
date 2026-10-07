use nf_contract::identity::*;
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::*,
    signing::*,
};
use nf_kernel::supplies::*;
use nf_store::supplies::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Scratch(PathBuf);
impl Scratch {
    pub fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!(
            "supplies-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn db(&self) -> PathBuf {
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
pub struct Fixture {
    pub policy: SuppliesPolicy,
    pub membership: MembershipState,
    pub issuer: PublicIdentity,
    pub beneficiary: PublicIdentity,
    issuer_key: SecretSeed,
    beneficiary_key: SecretSeed,
}
impl Fixture {
    pub fn new() -> Self {
        Self::new_with_roles(Roles::PLAYER)
    }
    pub fn new_with_roles(roles: Roles) -> Self {
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let (issuer, account, key) = generate_identity(b"supplies-issuer".to_vec()).unwrap();
        let (beneficiary, other_account, other_key) =
            generate_identity(b"supplies-beneficiary".to_vec()).unwrap();
        let state = MembershipState::bootstrap(scope, &issuer).unwrap();
        let invitation = Invitation {
            scope,
            id: [1; 16],
            issuer: issuer.account,
            recipient: beneficiary.clone(),
            roles,
            expires_at: 100,
            issued_revision: state.revision,
            reusable: false,
        };
        let proof = admission_proof(&invitation, &other_account, &other_key).unwrap();
        let signed = sign_invitation(invitation, &account).unwrap();
        let membership = state.redeem(&signed, &proof, 1).unwrap();
        let policy = SuppliesPolicy {
            universe: scope.universe,
            history: scope.history,
            ruleset: [3; 32],
            issuers: vec![IssuerRule {
                issuer: issuer.account,
                content: SUPPLIES_CONTENT,
                origin: Origin {
                    trust: TrustClass::Canonical,
                    lineage: [4; 32],
                },
                reason: IssuanceReason::AuthorityGrant,
                maximum: 1000,
            }],
            burners: vec![BurnRule {
                issuer: issuer.account,
                content: SUPPLIES_CONTENT,
                origin: Origin {
                    trust: TrustClass::Canonical,
                    lineage: [4; 32],
                },
                reason: BurnReason::AuthorityDestruction,
                maximum: 1000,
            }],
        };
        Self {
            policy,
            membership,
            issuer,
            beneficiary,
            issuer_key: key,
            beneficiary_key: other_key,
        }
    }
    pub fn issuance(&self) -> Issuance {
        Issuance {
            request: RequestId::from_bytes([8; 16]),
            issuance: OperationId::from_bytes([9; 16]),
            actor: self.issuer.account,
            device: self.issuer.device,
            beneficiary: self.beneficiary.account,
            universe: self.policy.universe,
            history: self.policy.history,
            policy: policy_digest(&self.policy).unwrap(),
            content: SUPPLIES_CONTENT,
            origin: self.policy.issuers[0].origin,
            reason: IssuanceReason::AuthorityGrant,
            amount: 25,
        }
    }
    pub fn query(&self) -> BalanceQuery {
        BalanceQuery {
            actor: self.beneficiary.account,
            device: self.beneficiary.device,
            owner: self.beneficiary.account,
            universe: self.policy.universe,
            history: self.policy.history,
            content: SUPPLIES_CONTENT,
            origin: self.policy.issuers[0].origin,
        }
    }
    pub fn attempt(
        &self,
        store: &mut SuppliesStore,
        request: ChallengeRequest<'_>,
    ) -> ProofAttempt {
        let issued = store.issue_challenge(request).unwrap();
        let mut proof = issued.template;
        let key = if proof.account == self.issuer.account {
            &self.issuer_key
        } else {
            assert_eq!(proof.account, self.beneficiary.account);
            &self.beneficiary_key
        };
        proof.signature = key.sign(&device_digest(&proof).unwrap());
        ProofAttempt {
            ticket: issued.ticket,
            proof,
        }
    }
}
