#![allow(dead_code)]
use crate::miniature::*;
use nf_contract::identity::*;
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::*,
    signing::*,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!(
            "miniature-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn db(&self) -> PathBuf {
        self.0.join("history.sqlite")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        assert!(self.0.starts_with(root));
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub struct Signer(pub SecretSeed);
impl BootstrapSigner for Signer {
    fn sign(&mut self, template: &DeviceProof) -> Result<DeviceProof, MiniatureStoreError> {
        let mut proof = template.clone();
        proof.signature = self.0.sign(&device_digest(&proof)?);
        Ok(proof)
    }
}
pub struct Fixture {
    pub spec: MiniatureGenesisSpec,
    pub policy: BootstrapPolicy,
    pub membership: MembershipState,
    pub signer: Signer,
    pub other_signers: Vec<(PublicIdentity, Signer)>,
}
impl Fixture {
    pub fn new() -> Self {
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let (owner, owner_key, owner_device) =
            generate_identity(b"miniature-owner".to_vec()).unwrap();
        let mut membership = MembershipState::bootstrap(scope, &owner).unwrap();
        let mut accounts = vec![owner.account];
        let mut other_signers = Vec::new();
        for n in 1..=2 {
            let (public, account, device) = generate_identity(vec![n]).unwrap();
            let invitation = Invitation {
                scope,
                id: [n; 16],
                issuer: owner.account,
                recipient: public.clone(),
                roles: Roles::PLAYER,
                expires_at: 100,
                issued_revision: membership.revision,
                reusable: false,
            };
            let proof = admission_proof(&invitation, &account, &device).unwrap();
            let signed = sign_invitation(invitation, &owner_key).unwrap();
            membership = membership.redeem(&signed, &proof, 1).unwrap();
            accounts.push(public.account);
            other_signers.push((public, Signer(device)));
        }
        let controllers: [AccountId; 3] = accounts.try_into().unwrap();
        let body = nf_identity::codec::encode_state(&membership).unwrap();
        use sha2::{Digest, Sha256};
        Self {
            spec: MiniatureGenesisSpec {
                genesis: nf_world::Genesis {
                    universe: scope.universe,
                    history: scope.history,
                    seed: [3; 32],
                    accounts: controllers,
                },
                aggregate: AggregateId::from_bytes([4; 16]),
                provider: ProviderId::from_bytes([5; 16]),
                provider_aggregate: AggregateId::from_bytes([6; 16]),
            },
            policy: BootstrapPolicy {
                scope,
                membership_digest: Sha256::digest(body).into(),
                owner,
                controllers,
                auth: AuthConfig::default(),
            },
            membership,
            signer: Signer(owner_device),
            other_signers,
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        nf_identity::codec::encode_state(&self.membership).unwrap()
    }
}

pub fn claim(store: &mut MiniatureStore, f: &mut Fixture) {
    let issued = store
        .issue_challenge(ChallengeRequest::Claim {
            actor: f.policy.owner.account,
            device: f.policy.owner.device,
        })
        .unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    store
        .claim_authority(ProofAttempt {
            ticket: issued.ticket,
            proof,
        })
        .unwrap();
}
pub fn intent(
    world: &nf_kernel::miniature::MiniatureWorld,
    owner: &PublicIdentity,
    n: u8,
) -> nf_kernel::miniature::MiniatureIntent {
    let g = world.component().genesis();
    let m = world.metadata();
    nf_kernel::miniature::MiniatureIntent {
        request: RequestId::from_bytes([n; 16]),
        operation: OperationId::from_bytes([n; 16]),
        job: JobId::from_bytes([n; 16]),
        actor: owner.account,
        device: owner.device,
        universe: g.universe,
        history: g.history,
        provider: m.provider,
        expected: [
            (m.aggregate, world.component().revision()),
            (m.provider_aggregate, m.provider_revision),
        ]
        .into(),
        action: nf_world::WorldAction::CreateColony {
            site: world
                .component()
                .markets()
                .iter()
                .find(|m| m.ordinal == 1)
                .unwrap()
                .id,
            faction: world
                .component()
                .factions()
                .iter()
                .find(|x| x.account == owner.account)
                .unwrap()
                .id,
        },
    }
}
