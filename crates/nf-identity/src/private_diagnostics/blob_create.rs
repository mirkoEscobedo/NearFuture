use super::{PrivateCause, PrivateFailure, PrivateOperation, PrivateResult, PrivateStage};
use crate::{model::IdentityError, private_storage::PrivateVault};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Write};
use zeroize::Zeroizing;
impl PrivateVault {
    /// Executes immutable blob creation once; no recovery or overwrite is attempted.
    pub fn create_private_blob_detailed(&self, name: &str, bytes: &[u8]) -> PrivateResult<()> {
        let path = self.blob_path(name).map_err(|_| {
            PrivateFailure::with_cause(
                PrivateOperation::BlobCreate,
                PrivateStage::Name,
                PrivateCause::Refused,
            )
        })?;
        if bytes.len() > 4096 {
            return Err(PrivateFailure::legacy(IdentityError::Limit));
        }
        super::access_cause(&self.root, false).map_err(|cause| {
            PrivateFailure::with_cause(
                PrivateOperation::BlobCreate,
                PrivateStage::RootAccess,
                cause,
            )
        })?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path).map_err(|error| {
            PrivateFailure::io(PrivateOperation::BlobCreate, PrivateStage::CreateNew, error)
        })?;
        super::access_cause(&path, true).map_err(|cause| {
            PrivateFailure::with_cause(
                PrivateOperation::BlobCreate,
                PrivateStage::EntryAccess,
                cause,
            )
        })?;
        let mut record = Zeroizing::new(Vec::with_capacity(46 + bytes.len()));
        record.extend_from_slice(b"NF-BLOB-1\0");
        record.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        record.extend_from_slice(bytes);
        let checksum = Sha256::digest(&*record);
        record.extend_from_slice(&checksum);
        file.write_all(&record).map_err(|error| {
            PrivateFailure::io(PrivateOperation::BlobCreate, PrivateStage::Write, error)
        })?;
        file.sync_all().map_err(|error| {
            PrivateFailure::io(PrivateOperation::BlobCreate, PrivateStage::Sync, error)
        })?;
        #[cfg(unix)]
        {
            let directory = std::fs::File::open(&self.root).map_err(|error| {
                PrivateFailure::io(
                    PrivateOperation::BlobCreate,
                    PrivateStage::DirectoryOpen,
                    error,
                )
            })?;
            directory.sync_all().map_err(|error| {
                PrivateFailure::io(
                    PrivateOperation::BlobCreate,
                    PrivateStage::DirectorySync,
                    error,
                )
            })?;
        }
        Ok(())
    }
}
