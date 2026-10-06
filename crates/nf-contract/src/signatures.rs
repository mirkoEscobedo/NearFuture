//! Standard Ed25519 over a validated NF-CANON-1 SHA-256 digest.
//! Authentication, authorization, key generation and key storage belong to the runtime shell.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

pub struct DigestSignature {
    pub public_key: [u8; 32],
    pub signature: [u8; 64],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSignature;

pub fn sign_digest(seed: &[u8; 32], digest: &[u8; 32]) -> DigestSignature {
    let key = SigningKey::from_bytes(seed);
    DigestSignature {
        public_key: key.verifying_key().to_bytes(),
        signature: key.sign(digest).to_bytes(),
    }
}
pub fn verify_digest(
    public_key: &[u8; 32],
    digest: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), InvalidSignature> {
    let key = admitted_point(public_key)?;
    let r: [u8; 32] = signature[..32].try_into().map_err(|_| InvalidSignature)?;
    admitted_point(&r)?;
    key.verify_strict(digest, &Signature::from_bytes(signature))
        .map_err(|_| InvalidSignature)
}

// Point parsing/order/canonical checks use the maintained curve implementation.
// Java's maintained full-point validation enforces this same closed profile.
fn admitted_point(bytes: &[u8; 32]) -> Result<VerifyingKey, InvalidSignature> {
    let key = VerifyingKey::from_bytes(bytes).map_err(|_| InvalidSignature)?;
    let point = key.to_edwards();
    if point.is_small_order() || !point.is_torsion_free() || point.compress().to_bytes() != *bytes {
        return Err(InvalidSignature);
    }
    Ok(key)
}
