use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, IdentityError, PublicIdentity, Scope},
    private_storage::PrivateVault,
    signing::device_digest,
};
use std::path::Path;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignerError {
    Unsupported,
    Identity(IdentityError),
    WrongIdentity,
    WrongTemplate,
}
/// Private local signer. Opening never creates a vault or replacement key.
/// A supplied public identity does not establish membership; the owner derives it from guarded Store reads.
pub struct VaultSigner {
    scope: Scope,
    public: PublicIdentity,
    device_key: SecretSeed,
}
impl VaultSigner {
    pub fn open(
        vault: &Path,
        game_save_root: &Path,
        scope: Scope,
        expected: &PublicIdentity,
    ) -> Result<Self, SignerError> {
        if expected.peer.is_empty() || expected.peer.len() > 128 {
            return Err(SignerError::WrongIdentity);
        }
        let private = PrivateVault::open(vault, game_save_root).map_err(SignerError::Identity)?;
        let local = private
            .load_membership_keys(expected)
            .map_err(SignerError::Identity)?;
        if local.public != *expected {
            return Err(SignerError::WrongIdentity);
        }
        Ok(Self {
            scope,
            public: local.public,
            device_key: local.device_key,
        })
    }
    pub fn public(&self) -> &PublicIdentity {
        &self.public
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    /// Sign only an unsigned template for this exact local scope/device/principal/peer.
    /// Freshness, purpose and current policy remain guarded by the issuing Store.
    pub fn sign(&self, template: &DeviceProof) -> Result<DeviceProof, SignerError> {
        if template.scope != self.scope
            || template.account != self.public.account
            || template.device != self.public.device
            || template.peer != self.public.peer
            || template.signature != [0; 64]
        {
            return Err(SignerError::WrongTemplate);
        }
        let mut proof = template.clone();
        proof.signature = self
            .device_key
            .sign(&device_digest(template).map_err(SignerError::Identity)?);
        Ok(proof)
    }
}
impl nf_store::miniature::BootstrapSigner for VaultSigner {
    fn sign(
        &mut self,
        template: &DeviceProof,
    ) -> Result<DeviceProof, nf_store::miniature::MiniatureStoreError> {
        VaultSigner::sign(self, template).map_err(|error| match error {
            SignerError::Identity(error) => {
                nf_store::miniature::MiniatureStoreError::Identity(error)
            }
            _ => nf_store::miniature::MiniatureStoreError::Unauthorized,
        })
    }
}
