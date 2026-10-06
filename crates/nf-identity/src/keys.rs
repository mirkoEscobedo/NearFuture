use crate::model::*;
use nf_contract::identity::{AccountId, DeviceId};
use zeroize::Zeroizing;

/// Owned private seed; never serializes or exposes a Debug representation.
pub struct SecretSeed(Zeroizing<[u8; 32]>);
impl SecretSeed {
    pub(crate) fn private_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub(crate) fn from_private_slice(bytes: &[u8]) -> Result<Self, IdentityError> {
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| IdentityError::MissingLocalState)?;
        Ok(Self(Zeroizing::new(seed)))
    }
    pub fn generate() -> Result<Self, IdentityError> {
        let mut seed = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *seed).map_err(|_| IdentityError::Entropy)?;
        Ok(Self(seed))
    }
    pub fn public_key(&self) -> [u8; 32] {
        nf_contract::signatures::sign_digest(&self.0, &[0; 32]).public_key
    }
    pub fn sign(&self, digest: &[u8; 32]) -> [u8; 64] {
        nf_contract::signatures::sign_digest(&self.0, digest).signature
    }
}
impl std::fmt::Debug for SecretSeed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretSeed(REDACTED)")
    }
}
pub fn random_id() -> Result<[u8; 16], IdentityError> {
    let mut id = [0; 16];
    getrandom::fill(&mut id).map_err(|_| IdentityError::Entropy)?;
    Ok(id)
}
pub fn generate_identity(
    peer: Vec<u8>,
) -> Result<(PublicIdentity, SecretSeed, SecretSeed), IdentityError> {
    if peer.is_empty() || peer.len() > 128 {
        return Err(IdentityError::Limit);
    }
    let account = SecretSeed::generate()?;
    let device = SecretSeed::generate()?;
    let public = PublicIdentity {
        account: AccountId::from_bytes(random_id()?),
        device: DeviceId::from_bytes(random_id()?),
        account_key: account.public_key(),
        device_key: device.public_key(),
        peer,
    };
    Ok((public, account, device))
}
