use nf_contract::identity::{BranchId, CampaignId, HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::{
        DeviceRevocation, Invitation, MembershipRepository, MembershipState, PublicIdentity, Roles,
        Scope,
    },
    persistence::redeem_persisted,
    rotation::revocation_digest,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::registration::{
    AllowedBranch, BranchRegistrar, CampaignBinding, KnownRegistrationFrontier, RegisterBranch,
    RegisteredBranch, RegistrationMode, RegistrationPolicy,
    lease::{
        AdmissionMode, AdmissionPolicy, AdmitLease, ClientSessionId, KnownAdmissionFrontier,
        LeaseStore, ProofAttempt,
    },
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Fixture {
    root: PathBuf,
    pub registration_policy: RegistrationPolicy,
    pub admission_policy: AdmissionPolicy,
    founder: PublicIdentity,
    founder_key: SecretSeed,
    player: PublicIdentity,
    account_key: SecretSeed,
    device_key: SecretSeed,
}
impl Fixture {
    pub fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!(
            "lease-revocation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let (founder, founder_key, _) = generate_identity(b"lease-founder".to_vec()).unwrap();
        let (player, account_key, device_key) =
            generate_identity(b"lease-player".to_vec()).unwrap();
        let registration_policy = RegistrationPolicy {
            scope: Scope {
                universe: UniverseId::from_bytes([31; 16]),
                history: HistoryId::from_bytes([32; 16]),
            },
            mode: RegistrationMode::HeadlessRegistrationOnly,
            allowed: vec![AllowedBranch {
                campaign: CampaignId::from_bytes([33; 16]),
                branch: BranchId::from_bytes([34; 16]),
                account: player.account,
            }],
        };
        let admission_policy = AdmissionPolicy {
            mode: AdmissionMode::HeadlessLeaseOnly,
            registration_policy_digest: registration_policy.digest().unwrap(),
        };
        Self {
            root,
            registration_policy,
            admission_policy,
            founder,
            founder_key,
            player,
            account_key,
            device_key,
        }
    }
    pub fn db(&self) -> PathBuf {
        self.root.join("lease.sqlite")
    }
    pub fn register(&self) -> (RegisteredBranch, KnownRegistrationFrontier) {
        let founder =
            MembershipState::bootstrap(self.registration_policy.scope, &self.founder).unwrap();
        let mut registrar =
            BranchRegistrar::create(self.db(), &self.registration_policy, &founder).unwrap();
        let invitation = Invitation {
            scope: self.registration_policy.scope,
            id: [36; 16],
            issuer: self.founder.account,
            recipient: self.player.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: founder.revision,
            reusable: false,
        };
        let proof = admission_proof(&invitation, &self.account_key, &self.device_key).unwrap();
        let invitation = sign_invitation(invitation, &self.founder_key).unwrap();
        let member = redeem_persisted(
            &mut registrar,
            self.registration_policy.scope,
            &invitation,
            &proof,
            1,
        )
        .unwrap();
        assert_eq!(member.revision, 1);
        assert_eq!(
            member.accounts.get(&self.player.account).unwrap().roles,
            Roles::PLAYER
        );
        assert_eq!(
            registrar
                .load_membership(self.registration_policy.scope)
                .unwrap(),
            Some(member)
        );
        let request = RegisterBranch {
            request: RequestId::from_bytes([35; 16]),
            scope: self.registration_policy.scope,
            campaign: CampaignId::from_bytes([33; 16]),
            branch: BranchId::from_bytes([34; 16]),
            account: self.player.account,
            device: self.player.device,
            policy_digest: self.registration_policy.digest().unwrap(),
        };
        let issued = registrar.issue_registration_challenge(&request).unwrap();
        let mut proof = issued.template;
        proof.signature = self.device_key.sign(&device_digest(&proof).unwrap());
        let registered = registrar
            .register_branch(
                &request,
                nf_store::registration::ProofAttempt {
                    ticket: issued.ticket,
                    proof,
                },
            )
            .unwrap();
        let expected = RegisteredBranch {
            binding: CampaignBinding {
                scope: request.scope,
                campaign: request.campaign,
                branch: request.branch,
                account: request.account,
            },
            original_request: request.request,
            revision: 1,
        };
        assert_eq!(registered, expected);
        let known = registrar.known_frontier().unwrap();
        assert_eq!((known.revision, known.minimum_membership_revision), (1, 1));
        assert_ne!(known.head, [0; 32]);
        drop(registrar);
        (registered, known)
    }
    pub fn request(
        &self,
        registration: RegisteredBranch,
        known_registration: KnownRegistrationFrontier,
        known_admission: KnownAdmissionFrontier,
    ) -> AdmitLease {
        AdmitLease {
            request: RequestId::from_bytes([40; 16]),
            binding: registration.binding,
            device: self.player.device,
            session: ClientSessionId::from_bytes([41; 16]).unwrap(),
            registration,
            known_registration,
            known_admission,
            policy_digest: self.admission_policy.digest(),
        }
    }
    pub fn signed_self_revocation(&self, frontier: u64) -> (DeviceRevocation, [u8; 64]) {
        let change = DeviceRevocation {
            scope: self.registration_policy.scope,
            issuer: self.player.account,
            device: self.player.device,
            frontier,
        };
        let signature = self.account_key.sign(&revocation_digest(&change));
        (change, signature)
    }
    pub fn attempt(&self, owner: &mut LeaseStore, request: &AdmitLease) -> ProofAttempt {
        let issued = owner
            .issue_admission_challenge(request)
            .expect("actual fresh distinct admission challenge");
        let mut proof = issued.template;
        assert_eq!(
            (proof.account, proof.device, proof.frontier),
            (self.player.account, self.player.device, 1)
        );
        proof.signature = self.device_key.sign(&device_digest(&proof).unwrap());
        ProofAttempt {
            ticket: issued.ticket,
            proof,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        assert!(self.root.starts_with(&parent));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
