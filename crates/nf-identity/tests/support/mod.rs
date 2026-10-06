#![allow(dead_code)] // Shared scenario support is compiled separately by each behavior test.
use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::{
    keys::{SecretSeed, generate_identity, random_id},
    model::*,
    signing::*,
};
pub struct Community {
    pub state: MembershipState,
    pub owner: PublicIdentity,
    pub owner_key: SecretSeed,
    pub recruit: PublicIdentity,
    pub account: SecretSeed,
    pub device: SecretSeed,
}
impl Community {
    pub fn new() -> Self {
        let (owner, owner_key, _) = generate_identity(vec![1, 2]).unwrap();
        let (recruit, account, device) = generate_identity(vec![3, 4]).unwrap();
        let scope = Scope {
            universe: UniverseId::from_bytes(random_id().unwrap()),
            history: HistoryId::from_bytes(random_id().unwrap()),
        };
        Self {
            state: MembershipState::bootstrap(scope, &owner).unwrap(),
            owner,
            owner_key,
            recruit,
            account,
            device,
        }
    }
    pub fn invitation(&self) -> SignedInvitation {
        sign_invitation(
            Invitation {
                scope: self.state.scope,
                id: random_id().unwrap(),
                issuer: self.owner.account,
                recipient: self.recruit.clone(),
                roles: Roles::PLAYER,
                expires_at: 100,
                issued_revision: self.state.revision,
                reusable: false,
            },
            &self.owner_key,
        )
        .unwrap()
    }
    pub fn proof(&self, invitation: &Invitation) -> AdmissionProof {
        admission_proof(invitation, &self.account, &self.device).unwrap()
    }
}
