use crate::{model::IdentityError, private_blob::read_blob_payload, private_storage::PrivateVault};
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
        self.scan_optional_private_blobs_with_access(names, visit, checked_access)
    }
    pub(crate) fn scan_optional_private_blobs_with_access(
        &self,
        names: &[&str],
        mut visit: impl for<'a> FnMut(usize, Option<&'a [u8]>) -> Result<(), IdentityError>,
        mut access: impl FnMut(&[&Path]) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        if names.is_empty() || names.len() > 64 {
            return Err(IdentityError::Limit);
        }
        let mut paths = Vec::with_capacity(names.len());
        checked_encoding(&self.root)?;
        for (index, name) in names.iter().enumerate() {
            if names[..index]
                .iter()
                .any(|previous| previous.eq_ignore_ascii_case(name))
            {
                return Err(IdentityError::PrivateStorage);
            }
            let path = self.blob_path(name)?;
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
                Err(_) => return Err(IdentityError::PrivateStorage),
            }
        }
        let mut admitted_paths = Vec::with_capacity(paths.len() + 1);
        admitted_paths.push(self.root.as_path());
        for (path, &exists) in paths.iter().zip(&present) {
            if exists {
                admitted_paths.push(path.as_path());
            }
        }
        access(&admitted_paths)?;
        checked_root_type(&self.root)?;
        for (index, (path, exists)) in paths.iter().zip(present).enumerate() {
            if exists {
                let bytes = Zeroizing::new(read_blob_payload(path)?);
                visit(index, Some(&bytes))?;
            } else {
                visit(index, None)?;
            }
        }
        access(&[self.root.as_path()])?;
        checked_root_type(&self.root)
    }
}
fn checked_encoding(path: &Path) -> Result<(), IdentityError> {
    let text = path.to_str().ok_or(IdentityError::PrivateStorage)?;
    if text.len() > 4096 || text.bytes().any(|value| matches!(value, 0 | b'\r' | b'\n')) {
        return Err(IdentityError::Limit);
    }
    Ok(())
}
fn checked_root_type(path: &Path) -> Result<(), IdentityError> {
    let entry = fs::symlink_metadata(path).map_err(|_| IdentityError::PrivateStorage)?;
    checked_not_link(&entry)?;
    if !entry.is_dir() {
        return Err(IdentityError::PrivateStorage);
    }
    Ok(())
}
fn checked_file_type(entry: &fs::Metadata) -> Result<(), IdentityError> {
    checked_not_link(entry)?;
    if !entry.is_file() {
        return Err(IdentityError::PrivateStorage);
    }
    Ok(())
}
fn checked_not_link(entry: &fs::Metadata) -> Result<(), IdentityError> {
    if entry.file_type().is_symlink() {
        return Err(IdentityError::PrivateStorage);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if entry.file_attributes() & 0x400 != 0 {
            return Err(IdentityError::PrivateStorage);
        }
    }
    Ok(())
}
fn checked_access(paths: &[&Path]) -> Result<(), IdentityError> {
    #[cfg(windows)]
    {
        crate::private_blob_acl::check(paths)
    }
    #[cfg(not(windows))]
    {
        for path in paths {
            crate::private_storage::private_access(path, false)?;
        }
        Ok(())
    }
}
