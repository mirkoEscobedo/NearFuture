use super::{PrivateFailure, PrivateOperation, PrivateResult, PrivateStage};
use crate::private_storage::{LocalIdentity, PrivateVault};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Write};
use zeroize::Zeroizing;

impl PrivateVault {
    /// Executes identity creation once and returns only this operation's failure.
    pub fn create_identity_detailed(&self, peer: Vec<u8>) -> PrivateResult<LocalIdentity> {
        super::access_cause(&self.root, false).map_err(|cause| {
            PrivateFailure::with_cause(
                PrivateOperation::IdentityCreate,
                PrivateStage::RootAccess,
                cause,
            )
        })?;
        let (public, account_key, device_key) =
            crate::keys::generate_identity(peer).map_err(PrivateFailure::legacy)?;
        let path = self.root.join("identity-key-v1");
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path).map_err(|error| {
            PrivateFailure::io(
                PrivateOperation::IdentityCreate,
                PrivateStage::CreateNew,
                error,
            )
        })?;
        super::access_cause(&path, true).map_err(|cause| {
            PrivateFailure::with_cause(
                PrivateOperation::IdentityCreate,
                PrivateStage::EntryAccess,
                cause,
            )
        })?;
        let mut record = Zeroizing::new(Vec::with_capacity(205));
        record.extend_from_slice(b"NF-PRIVATE-1\0");
        record.extend_from_slice(public.account.as_bytes());
        record.extend_from_slice(public.device.as_bytes());
        record.extend_from_slice(&public.account_key);
        record.extend_from_slice(&public.device_key);
        record.extend_from_slice(account_key.private_bytes());
        record.extend_from_slice(device_key.private_bytes());
        let checksum = Sha256::digest(&*record);
        record.extend_from_slice(&checksum);
        file.write_all(&record).map_err(|error| {
            PrivateFailure::io(PrivateOperation::IdentityCreate, PrivateStage::Write, error)
        })?;
        file.sync_all().map_err(|error| {
            PrivateFailure::io(PrivateOperation::IdentityCreate, PrivateStage::Sync, error)
        })?;
        #[cfg(unix)]
        {
            let directory = std::fs::File::open(&self.root).map_err(|error| {
                PrivateFailure::io(
                    PrivateOperation::IdentityCreate,
                    PrivateStage::DirectoryOpen,
                    error,
                )
            })?;
            directory.sync_all().map_err(|error| {
                PrivateFailure::io(
                    PrivateOperation::IdentityCreate,
                    PrivateStage::DirectorySync,
                    error,
                )
            })?;
        }
        Ok(LocalIdentity {
            public,
            account_key,
            device_key,
        })
    }
}
