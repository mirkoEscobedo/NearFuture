use crate::{
    codec::{Writer, encode_invitation},
    keys::SecretSeed,
    model::*,
};
use sha2::{Digest, Sha256};
pub fn invitation_digest(invitation: &Invitation) -> Result<[u8; 32], IdentityError> {
    Ok(Sha256::digest(encode_invitation(invitation)?).into())
}
pub fn sign_invitation(
    invitation: Invitation,
    key: &SecretSeed,
) -> Result<SignedInvitation, IdentityError> {
    let signature = key.sign(&invitation_digest(&invitation)?);
    Ok(SignedInvitation {
        invitation,
        signature,
    })
}
pub fn admission_proof(
    invitation: &Invitation,
    account: &SecretSeed,
    device: &SecretSeed,
) -> Result<AdmissionProof, IdentityError> {
    let digest = invitation_digest(invitation)?;
    Ok(AdmissionProof {
        account_signature: account.sign(&digest),
        device_signature: device.sign(&digest),
    })
}
pub fn device_digest(proof: &DeviceProof) -> Result<[u8; 32], IdentityError> {
    let mut out = Writer::new(5);
    out.scope(proof.scope);
    out.raw(proof.account.as_bytes());
    out.raw(proof.device.as_bytes());
    out.u64(proof.frontier);
    out.peer(&proof.peer)?;
    out.raw(&proof.challenge);
    Ok(Sha256::digest(out.0).into())
}
pub(crate) fn verify(
    key: &[u8; 32],
    digest: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), IdentityError> {
    nf_contract::signatures::verify_digest(key, digest, signature)
        .map_err(|_| IdentityError::Signature)
}
