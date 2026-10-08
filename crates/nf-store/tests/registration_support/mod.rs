use nf_contract::identity::{BranchId, CampaignId, HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::{Invitation, MembershipRepository, MembershipState, PublicIdentity, Roles, Scope},
    persistence::redeem_persisted,
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::registration::{
    AllowedBranch, BranchRegistrar, ProofAttempt, RegisterBranch, RegistrationMode,
    RegistrationPolicy,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    root: PathBuf,
    pub policy: RegistrationPolicy,
    founder: PublicIdentity,
    founder_account_key: SecretSeed,
    player: PublicIdentity,
    player_account_key: SecretSeed,
    player_device_key: SecretSeed,
}
impl Fixture {
    pub fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&parent).unwrap();
        let root = parent.join(format!(
            "registration-contract-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let (founder, founder_account_key, _) =
            generate_identity(b"registration-founder".to_vec()).unwrap();
        let (player, player_account_key, player_device_key) =
            generate_identity(b"registration-player".to_vec()).unwrap();
        let policy = RegistrationPolicy {
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
        Self {
            root,
            policy,
            founder,
            founder_account_key,
            player,
            player_account_key,
            player_device_key,
        }
    }
    pub fn db(&self) -> PathBuf {
        self.root.join("registration.sqlite")
    }
    pub fn create(&self) -> BranchRegistrar {
        let founder = MembershipState::bootstrap(self.policy.scope, &self.founder).unwrap();
        let mut registrar = BranchRegistrar::create(self.db(), &self.policy, &founder)
            .expect("actual separate registration profile initialized with founder membership");
        let invitation = Invitation {
            scope: self.policy.scope,
            id: [36; 16],
            issuer: self.founder.account,
            recipient: self.player.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: founder.revision,
            reusable: false,
        };
        let proof = admission_proof(
            &invitation,
            &self.player_account_key,
            &self.player_device_key,
        )
        .unwrap();
        let signed = sign_invitation(invitation, &self.founder_account_key).unwrap();
        let admitted = redeem_persisted(&mut registrar, self.policy.scope, &signed, &proof, 1)
            .expect("signed PLAYER invitation commits in the same registrar profile");
        assert_eq!(admitted.revision, 1);
        assert_eq!(
            admitted.accounts.get(&self.player.account).unwrap().roles,
            Roles::PLAYER
        );
        assert_eq!(
            registrar.load_membership(self.policy.scope).unwrap(),
            Some(admitted)
        );
        registrar
    }
    pub fn request(&self) -> RegisterBranch {
        RegisterBranch {
            request: RequestId::from_bytes([35; 16]),
            scope: self.policy.scope,
            campaign: CampaignId::from_bytes([33; 16]),
            branch: BranchId::from_bytes([34; 16]),
            account: self.player.account,
            device: self.player.device,
            policy_digest: self.policy.digest().unwrap(),
        }
    }
    pub fn attempt(
        &self,
        registrar: &mut BranchRegistrar,
        request: &RegisterBranch,
    ) -> ProofAttempt {
        let issued = registrar
            .issue_registration_challenge(request)
            .expect("actual fresh registration challenge");
        let mut proof = issued.template;
        assert_eq!(
            (proof.account, proof.device, proof.frontier),
            (self.player.account, self.player.device, 1)
        );
        proof.signature = self.player_device_key.sign(&device_digest(&proof).unwrap());
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
