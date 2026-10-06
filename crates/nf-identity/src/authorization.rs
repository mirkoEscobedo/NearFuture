use crate::{
    model::*,
    signing::{device_digest, verify},
};
impl MembershipState {
    /// The caller supplies the peer authenticated by its transport and a fresh one-use challenge.
    pub fn authorize(
        &self,
        proof: &DeviceProof,
        peer: &[u8],
        challenge: &[u8; 32],
        required_frontier: u64,
        operation: ProtectedOperation,
    ) -> Result<u64, IdentityError> {
        crate::codec::validate_state(self)?;
        if proof.scope != self.scope {
            return Err(IdentityError::Scope);
        }
        if self.revision < required_frontier || proof.frontier != self.revision {
            return Err(IdentityError::Frontier);
        }
        let device = self
            .devices
            .get(&proof.device)
            .ok_or(IdentityError::UnknownDevice)?;
        if device.revoked {
            return Err(IdentityError::Revoked);
        }
        if device.account != proof.account
            || device.peer != peer
            || proof.peer != peer
            || &proof.challenge != challenge
        {
            return Err(IdentityError::Signature);
        }
        verify(&device.key, &device_digest(proof)?, &proof.signature)?;
        let account = self
            .accounts
            .get(&proof.account)
            .ok_or(IdentityError::UnknownAccount)?;
        let required = match operation {
            ProtectedOperation::Economic | ProtectedOperation::Chat => Roles::PLAYER,
            ProtectedOperation::Administration => Roles::MODERATOR,
            ProtectedOperation::Worker => Roles::WORKER,
        };
        if !account.roles.contains(required) {
            return Err(IdentityError::RolePolicy);
        }
        Ok(self.revision)
    }
}
