use crate::{
    model::IdentityError,
    private_blob::read_blob_payload_detailed,
    private_diagnostics::{
        PrivateCause, PrivateFailure, PrivateOperation, PrivateResult, PrivateStage,
    },
    private_storage::PrivateVault,
};
use std::{fs, path::Path};
use zeroize::Zeroizing;
impl PrivateVault {
    /// One complete provisional pass over at most64 closed names. Publish only after final Ok.
    /// The trusted visitor must not mutate the vault or retain private payload copies.
    /// This preserves the cooperative exclusive-lifecycle assumption, not an atomic FS snapshot.
    pub fn scan_optional_private_blobs(
        &self,
        names: &[&str],
        visit: impl for<'a> FnMut(usize, Option<&'a [u8]>) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        self.scan_optional_private_blobs_detailed(names, visit)
            .map_err(PrivateFailure::identity_error)
    }
    /// Returned payloads remain provisional until the complete pass succeeds.
    pub fn scan_optional_private_blobs_detailed(
        &self,
        names: &[&str],
        mut visit: impl for<'a> FnMut(usize, Option<&'a [u8]>) -> Result<(), IdentityError>,
    ) -> PrivateResult<()> {
        if names.is_empty() || names.len() > 64 {
            return Err(PrivateFailure::legacy(IdentityError::Limit));
        }
        let mut paths = Vec::with_capacity(names.len());
        checked_encoding(&self.root)?;
        for (index, name) in names.iter().enumerate() {
            if names[..index]
                .iter()
                .any(|previous| previous.eq_ignore_ascii_case(name))
            {
                return Err(refused(PrivateStage::Name));
            }
            let path = self
                .blob_path(name)
                .map_err(|_| refused(PrivateStage::Name))?;
            checked_encoding(&path)?;
            paths.push(path);
        }
        checked_root_type(&self.root)?;
        let mut present = Vec::with_capacity(paths.len());
        for path in &paths {
            match fs::symlink_metadata(path) {
                Ok(entry) => {
                    checked_file_type(&entry)?;
                    present.push(true);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => present.push(false),
                Err(error) => return Err(io(PrivateStage::EntryMetadata, error)),
            }
        }
        let mut admitted_paths = Vec::with_capacity(paths.len() + 1);
        admitted_paths.push(self.root.as_path());
        for (path, &exists) in paths.iter().zip(&present) {
            if exists {
                admitted_paths.push(path.as_path());
            }
        }
        checked_access(&admitted_paths, PrivateStage::ProvisionalAccess)?;
        checked_root_type(&self.root)?;
        for (index, (path, exists)) in paths.iter().zip(present).enumerate() {
            if exists {
                let bytes = Zeroizing::new(read_blob_payload_detailed(path)?);
                visit(index, Some(&bytes)).map_err(PrivateFailure::legacy)?;
            } else {
                visit(index, None).map_err(PrivateFailure::legacy)?;
            }
        }
        checked_access(&[self.root.as_path()], PrivateStage::FinalAccess)?;
        checked_root_type(&self.root)
    }
}
fn refused(stage: PrivateStage) -> PrivateFailure {
    PrivateFailure::with_cause(PrivateOperation::BlobScan, stage, PrivateCause::Refused)
}
fn io(stage: PrivateStage, error: std::io::Error) -> PrivateFailure {
    PrivateFailure::io(PrivateOperation::BlobScan, stage, error)
}
fn checked_encoding(path: &Path) -> PrivateResult<()> {
    let text = path.to_str().ok_or(refused(PrivateStage::Encoding))?;
    if text.len() > 4096 || text.bytes().any(|value| matches!(value, 0 | b'\r' | b'\n')) {
        return Err(PrivateFailure::legacy(IdentityError::Limit));
    }
    Ok(())
}
fn checked_root_type(path: &Path) -> PrivateResult<()> {
    let entry =
        fs::symlink_metadata(path).map_err(|error| io(PrivateStage::RootMetadata, error))?;
    checked_not_link(&entry, PrivateStage::RootMetadata)?;
    if !entry.is_dir() {
        return Err(refused(PrivateStage::RootMetadata));
    }
    Ok(())
}
fn checked_file_type(entry: &fs::Metadata) -> PrivateResult<()> {
    checked_not_link(entry, PrivateStage::EntryType)?;
    if !entry.is_file() {
        return Err(refused(PrivateStage::EntryType));
    }
    Ok(())
}
fn checked_not_link(entry: &fs::Metadata, stage: PrivateStage) -> PrivateResult<()> {
    if entry.file_type().is_symlink() {
        return Err(refused(stage));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if entry.file_attributes() & 0x400 != 0 {
            return Err(refused(stage));
        }
    }
    Ok(())
}
fn checked_access(paths: &[&Path], stage: PrivateStage) -> PrivateResult<()> {
    #[cfg(windows)]
    {
        crate::private_blob_acl::check(paths, stage)
    }
    #[cfg(not(windows))]
    {
        for path in paths {
            crate::private_diagnostics::access_cause(path, false).map_err(|cause| {
                PrivateFailure::with_cause(PrivateOperation::BlobScan, stage, cause)
            })?;
        }
        Ok(())
    }
}
