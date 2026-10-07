pub use crate::private_diagnostics::{
    HelperCause, HelperDiagnostic, HelperExitStage, HelperKind, HelperStep, PrivateCause,
    PrivateDiagnostic, PrivateFailure, PrivateOperation, PrivateResult, PrivateStage,
};
use crate::{keys::SecretSeed, model::*};
use nf_contract::identity::{AccountId, DeviceId};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub struct PrivateVault {
    pub(crate) root: PathBuf,
}
pub struct LocalIdentity {
    pub public: PublicIdentity,
    pub account_key: SecretSeed,
    pub device_key: SecretSeed,
}
impl PrivateVault {
    pub fn create(path: &Path, game_save_root: &Path) -> Result<Self, IdentityError> {
        fs::create_dir(path).map_err(|_| IdentityError::PrivateStorage)?;
        private_access(path, true)?;
        Self::open(path, game_save_root)
    }
    pub fn open(path: &Path, game_save_root: &Path) -> Result<Self, IdentityError> {
        private_access(path, false)?;
        let root = fs::canonicalize(path).map_err(|_| IdentityError::PrivateStorage)?;
        let saves = fs::canonicalize(game_save_root).map_err(|_| IdentityError::PrivateStorage)?;
        if root.starts_with(&saves) || !root.is_dir() {
            return Err(IdentityError::PrivateStorage);
        }
        Ok(Self { root })
    }
    pub fn create_identity(&self, peer: Vec<u8>) -> Result<LocalIdentity, IdentityError> {
        self.create_identity_detailed(peer)
            .map_err(crate::private_diagnostics::PrivateFailure::identity_error)
    }
    /// Never generates replacement keys when state is absent or damaged.
    pub fn load_identity(&self, peer: Vec<u8>) -> Result<LocalIdentity, IdentityError> {
        private_access(&self.root, false)?;
        let path = self.root.join("identity-key-v1");
        if !path.exists() {
            return Err(IdentityError::MissingLocalState);
        }
        private_access(&path, false)?;
        let mut file = fs::File::open(&path).map_err(|_| IdentityError::PrivateStorage)?;
        if file
            .metadata()
            .map_err(|_| IdentityError::PrivateStorage)?
            .len()
            != 205
        {
            return Err(IdentityError::MissingLocalState);
        }
        let mut record = Zeroizing::new([0u8; 205]);
        file.read_exact(&mut *record)
            .map_err(|_| IdentityError::MissingLocalState)?;
        if Sha256::digest(&record[..173]).as_slice() != &record[173..] {
            return Err(IdentityError::MissingLocalState);
        }
        if &record[..13] != b"NF-PRIVATE-1\0" || peer.is_empty() || peer.len() > 128 {
            return Err(IdentityError::MissingLocalState);
        }
        let account_key = SecretSeed::from_private_slice(&record[109..141])?;
        let device_key = SecretSeed::from_private_slice(&record[141..173])?;
        if account_key.public_key() != record[45..77] || device_key.public_key() != record[77..109]
        {
            return Err(IdentityError::MissingLocalState);
        }
        let public = PublicIdentity {
            account: AccountId::from_slice(&record[13..29])
                .map_err(|_| IdentityError::MissingLocalState)?,
            device: DeviceId::from_slice(&record[29..45])
                .map_err(|_| IdentityError::MissingLocalState)?,
            account_key: account_key.public_key(),
            device_key: device_key.public_key(),
            peer,
        };
        Ok(LocalIdentity {
            public,
            account_key,
            device_key,
        })
    }
}
pub(crate) fn private_access(path: &Path, initialize: bool) -> Result<(), IdentityError> {
    crate::private_diagnostics::access_cause(path, initialize)
        .map_err(|_| IdentityError::PrivateStorage)
}
