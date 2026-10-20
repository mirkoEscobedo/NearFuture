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
        nf_contract::signatures::public_key_from_seed(&self.0)
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

#[cfg(test)]
mod public_key_tests {
    use super::SecretSeed;
    use nf_contract::signatures::verify_digest;

    #[test]
    fn public_rfc_seed_derives_known_key_and_still_signs_exact_digest() {
        // Public RFC8032 test material, never a credential.
        let seed = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1c, 0xae, 0x7f, 0x60,
        ];
        let expected = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];
        let key = SecretSeed::from_private_slice(&seed).unwrap();
        assert_eq!(key.public_key(), expected);
        let digest = [7; 32];
        let signature = key.sign(&digest);
        assert!(verify_digest(&expected, &digest, &signature).is_ok());
        assert!(verify_digest(&expected, &[8; 32], &signature).is_err());
    }
}
