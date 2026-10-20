//! Trusted-local insertion; does not register an incoming byte or wire decoder.
use crate::{
    codec::{MAX_DEVICES, Writer, validate_state},
    model::{Device, DeviceEnrollment, DeviceEnrollmentProof, IdentityError, MembershipState},
    signing::verify,
};
use sha2::{Digest, Sha256};

/// Identity extension domain8/kind7/schema1; both proofs bind this exact purpose.
pub fn enrollment_digest(change: &DeviceEnrollment) -> Result<[u8; 32], IdentityError> {
    let mut out = Writer::new(7);
    out.scope(change.scope);
    out.raw(change.account.as_bytes());
    out.u64(change.frontier);
    out.raw(change.new_device.as_bytes());
    out.raw(&change.new_key);
    out.peer(&change.new_peer)?;
    Ok(Sha256::digest(out.0).into())
}
impl MembershipState {
    pub fn enroll_device(
        &self,
        change: &DeviceEnrollment,
        proof: &DeviceEnrollmentProof,
    ) -> Result<Self, IdentityError> {
        validate_state(self)?;
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
        if self.devices.contains_key(&change.new_device) {
            return Err(IdentityError::Conflict);
        }
        if self.devices.len() >= MAX_DEVICES {
            return Err(IdentityError::Limit);
        }
        let digest = enrollment_digest(change)?;
        verify(&account.key, &digest, &proof.account_signature)?;
        verify(&change.new_key, &digest, &proof.new_device_signature)?;
        let mut next = self.clone();
        next.revision = next.revision.checked_add(1).ok_or(IdentityError::Limit)?;
        next.devices.insert(
            change.new_device,
            Device {
                account: change.account,
                key: change.new_key,
                peer: change.new_peer.clone(),
                revoked: false,
            },
        );
        validate_state(&next)?;
        // Complete bounded state encoding before a shell may persist the transition.
        crate::codec::encode_state(&next)?;
        Ok(next)
    }
}
