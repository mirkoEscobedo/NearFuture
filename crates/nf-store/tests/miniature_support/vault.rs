#![allow(dead_code)]
use crate::miniature_support::{Scratch, Signer};
use nf_contract::identity::*;
use nf_identity::{keys::generate_identity, model::*, private_storage::PrivateVault, signing::*};
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
pub struct VaultFixture {
    pub scratch: Scratch,
    pub vault: PathBuf,
    pub saves: PathBuf,
    pub spec: MiniatureGenesisSpec,
    pub policy: BootstrapPolicy,
    pub membership: MembershipState,
    pub signer: Signer,
}
impl VaultFixture {
    pub fn new() -> Self {
        let scratch = Scratch::new();
        let saves = scratch.0.join("saves");
        std::fs::create_dir(&saves).unwrap();
        let vault = scratch.0.join("private-owner");
        let private = PrivateVault::create(&vault, &saves).unwrap();
        let local = private
            .create_identity(b"miniature-owned-worker".to_vec())
            .unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        };
        let mut membership = MembershipState::bootstrap(scope, &local.public).unwrap();
        let mut accounts = vec![local.public.account];
        for n in 1..=2 {
            let (public, account, device) = generate_identity(vec![n]).unwrap();
            let invitation = Invitation {
                scope,
                id: [n; 16],
                issuer: local.public.account,
                recipient: public.clone(),
                roles: Roles::PLAYER,
                expires_at: 100,
                issued_revision: membership.revision,
                reusable: false,
            };
            let proof = admission_proof(&invitation, &account, &device).unwrap();
            let signed = sign_invitation(invitation, &local.account_key).unwrap();
            membership = membership.redeem(&signed, &proof, 1).unwrap();
            accounts.push(public.account);
        }
        let controllers: [AccountId; 3] = accounts.try_into().unwrap();
        let body = nf_identity::codec::encode_state(&membership).unwrap();
        Self {
            scratch,
            vault,
            saves,
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
                owner: local.public,
                controllers,
                auth: AuthConfig::default(),
            },
            membership,
            signer: Signer(local.device_key),
        }
    }
    pub fn create(&mut self, path: &std::path::Path) -> MiniatureStore {
        let bytes = nf_identity::codec::encode_state(&self.membership).unwrap();
        MiniatureStore::create(
            path,
            self.spec,
            &bytes,
            self.policy.clone(),
            &mut self.signer,
        )
        .unwrap()
    }
}
pub fn load_signer(
    store: &mut MiniatureStore,
    vault: &std::path::Path,
    saves: &std::path::Path,
) -> Signer {
    use nf_identity::model::MembershipRepository;
    let g = store.world().component().genesis();
    let scope = Scope {
        universe: g.universe,
        history: g.history,
    };
    let membership = store.load_membership(scope).unwrap().unwrap();
    let account = g.accounts[0];
    let device = *membership
        .devices
        .iter()
        .find(|(_, d)| d.account == account && !d.revoked)
        .unwrap()
        .0;
    let expected = store.current_identity(account, device).unwrap();
    let local = PrivateVault::open(vault, saves)
        .unwrap()
        .load_identity(expected.peer.clone())
        .unwrap();
    assert_eq!(local.public, expected);
    Signer(local.device_key)
}
pub fn owner_attempt(
    store: &mut MiniatureStore,
    signer: &mut Signer,
    request: ChallengeRequest<'_>,
) -> ProofAttempt {
    let issued = store.issue_challenge(request).unwrap();
    let proof = signer.sign(&issued.template).unwrap();
    ProofAttempt {
        ticket: issued.ticket,
        proof,
    }
}
pub fn fresh_claim(store: &mut MiniatureStore, signer: &mut Signer) {
    use nf_identity::model::MembershipRepository;
    let g = store.world().component().genesis();
    let scope = Scope {
        universe: g.universe,
        history: g.history,
    };
    let member = store.load_membership(scope).unwrap().unwrap();
    let device = *member
        .devices
        .iter()
        .find(|(_, d)| d.account == g.accounts[0] && !d.revoked)
        .unwrap()
        .0;
    let attempt = owner_attempt(
        store,
        signer,
        ChallengeRequest::Claim {
            actor: g.accounts[0],
            device,
        },
    );
    store.claim_authority(attempt).unwrap();
}
pub fn fresh_resume(store: &mut MiniatureStore, signer: &mut Signer) {
    let ids: Vec<_> = store
        .pending()
        .unwrap()
        .intents()
        .iter()
        .map(|i| i.request)
        .collect();
    let attempts = ids
        .into_iter()
        .map(|id| owner_attempt(store, signer, ChallengeRequest::Resume(id)))
        .collect();
    store.resume_pending(attempts).unwrap();
}
pub fn activity(store: &mut MiniatureStore, signer: &mut Signer) {
    let a = store.authority().unwrap();
    let attempt = owner_attempt(
        store,
        signer,
        ChallengeRequest::Activity {
            actor: a.account,
            device: a.device,
        },
    );
    store.accept_activity(attempt).unwrap();
}
