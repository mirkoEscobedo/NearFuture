use crate::{codec::Writer, model::*};
use sha2::{Digest, Sha256};
pub fn rotation_digest(change: &DeviceRotation) -> Result<[u8; 32], IdentityError> {
    let mut out = Writer::new(2);
    out.scope(change.scope);
    out.raw(change.account.as_bytes());
    out.raw(change.device.as_bytes());
    out.u64(change.frontier);
    out.raw(&change.new_key);
    out.peer(&change.new_peer)?;
    Ok(Sha256::digest(out.0).into())
}
pub fn account_rotation_digest(change: &AccountRotation) -> [u8; 32] {
    let mut out = Writer::new(3);
    out.scope(change.scope);
    out.raw(change.account.as_bytes());
    out.u64(change.frontier);
    out.raw(&change.new_key);
    Sha256::digest(out.0).into()
}
pub fn revocation_digest(change: &DeviceRevocation) -> [u8; 32] {
    let mut out = Writer::new(4);
    out.scope(change.scope);
    out.raw(change.issuer.as_bytes());
    out.raw(change.device.as_bytes());
    out.u64(change.frontier);
    Sha256::digest(out.0).into()
}
impl MembershipState {
    pub fn rotate_account(
        &self,
        change: &AccountRotation,
        old_signature: &[u8; 64],
        new_signature: &[u8; 64],
    ) -> Result<Self, IdentityError> {
        crate::codec::validate_state(self)?;
        if change.scope != self.scope {
            return Err(IdentityError::Scope);
        }
        if change.frontier != self.revision {
            return Err(IdentityError::Frontier);
        }
        let account = self
            .accounts
            .get(&change.account)
            .ok_or(IdentityError::UnknownAccount)?;
        let digest = account_rotation_digest(change);
        crate::signing::verify(&account.key, &digest, old_signature)?;
        crate::signing::verify(&change.new_key, &digest, new_signature)?;
        let mut next = self.clone();
        next.revision = next.revision.checked_add(1).ok_or(IdentityError::Limit)?;
        next.accounts
            .get_mut(&change.account)
            .ok_or(IdentityError::UnknownAccount)?
            .key = change.new_key;
        Ok(next)
    }
    /// Lost account keys require an explicit new-history social recovery decision elsewhere.
    pub fn recover_lost_account(&self) -> Result<Self, IdentityError> {
        Err(IdentityError::LostKeyRecoveryRefused)
    }

    pub fn rotate_device(
        &self,
        change: &DeviceRotation,
        account_signature: &[u8; 64],
        device_signature: &[u8; 64],
    ) -> Result<Self, IdentityError> {
        crate::codec::validate_state(self)?;
        if change.scope != self.scope {
            return Err(IdentityError::Scope);
        }
        if change.frontier != self.revision {
            return Err(IdentityError::Frontier);
        }
        let account = self
            .accounts
            .get(&change.account)
            .ok_or(IdentityError::UnknownAccount)?;
        let device = self
            .devices
            .get(&change.device)
            .ok_or(IdentityError::UnknownDevice)?;
        if device.account != change.account {
            return Err(IdentityError::RolePolicy);
        }
        let digest = rotation_digest(change)?;
        crate::signing::verify(&account.key, &digest, account_signature)?;
        crate::signing::verify(&change.new_key, &digest, device_signature)?;
        let mut next = self.clone();
        next.revision = next.revision.checked_add(1).ok_or(IdentityError::Limit)?;
        let device = next
            .devices
            .get_mut(&change.device)
            .ok_or(IdentityError::UnknownDevice)?;
        device.key = change.new_key;
        device.peer = change.new_peer.clone();
        device.revoked = false;
        Ok(next)
    }

    pub fn revoke_device(
        &self,
        change: &DeviceRevocation,
        signature: &[u8; 64],
    ) -> Result<Self, IdentityError> {
        crate::codec::validate_state(self)?;
        if change.scope != self.scope {
            return Err(IdentityError::Scope);
        }
        if change.frontier != self.revision {
            return Err(IdentityError::Frontier);
        }
        let device = self
            .devices
            .get(&change.device)
            .ok_or(IdentityError::UnknownDevice)?;
        let issuer = self
            .accounts
            .get(&change.issuer)
            .ok_or(IdentityError::UnknownAccount)?;
        if change.issuer != device.account
            && change.issuer != self.owner
            && (!issuer.roles.contains(Roles::MODERATOR) || device.account == self.owner)
        {
            return Err(IdentityError::RolePolicy);
        }
        crate::signing::verify(&issuer.key, &revocation_digest(change), signature)?;
        let mut next = self.clone();
        next.revision = next.revision.checked_add(1).ok_or(IdentityError::Limit)?;
        next.devices
            .get_mut(&change.device)
            .ok_or(IdentityError::UnknownDevice)?
            .revoked = true;
        Ok(next)
    }
}
