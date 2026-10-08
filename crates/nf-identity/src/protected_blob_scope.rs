use crate::{model::IdentityError, private_storage::PrivateVault};
use std::path::Path;

/// One explicit protected-blob operation, with fresh admission at every existing cut.
/// Contains one Windows helper; Drop cancels it. No ACL result is cached.
pub struct ProtectedBlobScope<'a> {
    vault: &'a PrivateVault,
    #[cfg(windows)]
    session: Option<crate::private_acl_session::Session>,
}
impl PrivateVault {
    pub fn protected_blob_scope(&self) -> Result<ProtectedBlobScope<'_>, IdentityError> {
        Ok(ProtectedBlobScope {
            vault: self,
            #[cfg(windows)]
            session: Some(crate::private_acl_session::Session::start()?),
        })
    }
}
impl ProtectedBlobScope<'_> {
    fn access(&mut self, paths: &[&Path], initialize: bool) -> Result<(), IdentityError> {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            for path in paths {
                let entry =
                    std::fs::symlink_metadata(path).map_err(|_| IdentityError::PrivateStorage)?;
                if entry.file_attributes() & 0x400 != 0 {
                    return Err(IdentityError::PrivateStorage);
                }
            }
            self.session
                .as_mut()
                .ok_or(IdentityError::PrivateStorage)?
                .check(paths, initialize)
        }
        #[cfg(not(windows))]
        {
            for path in paths {
                crate::private_storage::private_access(path, initialize)?;
            }
            Ok(())
        }
    }
    fn refuse(&mut self) {
        #[cfg(windows)]
        {
            self.session.take();
        }
    }
    pub fn scan_optional_private_blobs(
        &mut self,
        names: &[&str],
        visit: impl for<'a> FnMut(usize, Option<&'a [u8]>) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        let vault = self.vault;
        let result = vault.scan_optional_private_blobs_with_access(names, visit, |paths| {
            self.access(paths, false)
        });
        if result.is_err() {
            self.refuse();
        }
        result
    }
    pub fn create_private_blob(&mut self, name: &str, bytes: &[u8]) -> Result<(), IdentityError> {
        let vault = self.vault;
        let result = vault.create_private_blob_with_access(name, bytes, |path, initialize| {
            self.access(&[path], initialize)
        });
        if result.is_err() {
            self.refuse();
        }
        result
    }
    /// Completes exactly the two fresh validation cuts of an unchanged receipt read.
    pub fn finish_readonly(self) -> Result<(), IdentityError> {
        #[cfg(windows)]
        {
            let mut scope = self;
            scope
                .session
                .take()
                .ok_or(IdentityError::PrivateStorage)?
                .finish_readonly()
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }
    /// Completes all six fresh cuts, including initialization at cut four.
    pub fn finish_append(self) -> Result<(), IdentityError> {
        #[cfg(windows)]
        {
            let mut scope = self;
            scope
                .session
                .take()
                .ok_or(IdentityError::PrivateStorage)?
                .finish_append()
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }
}
