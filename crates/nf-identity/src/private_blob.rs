use crate::{
    model::IdentityError,
    private_storage::{PrivateVault, private_access},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};
use zeroize::Zeroizing;

impl PrivateVault {
    pub fn create_private_blob(&self, name: &str, bytes: &[u8]) -> Result<(), IdentityError> {
        let path = self.blob_path(name)?;
        if bytes.len() > 4096 {
            return Err(IdentityError::Limit);
        }
        private_access(&self.root, false)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|_| IdentityError::PrivateStorage)?;
        private_access(&path, true)?;
        let mut record = Zeroizing::new(Vec::with_capacity(46 + bytes.len()));
        record.extend_from_slice(b"NF-BLOB-1\0");
        record.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        record.extend_from_slice(bytes);
        let checksum = Sha256::digest(&*record);
        record.extend_from_slice(&checksum);
        file.write_all(&record)
            .and_then(|()| file.sync_all())
            .map_err(|_| IdentityError::PrivateStorage)?;
        #[cfg(unix)]
        {
            fs::File::open(&self.root)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| IdentityError::PrivateStorage)?;
        }
        Ok(())
    }
    /// Caller owns returned secret bytes and must keep them out of diagnostics.
    pub fn read_private_blob(&self, name: &str) -> Result<Vec<u8>, IdentityError> {
        let path = self.blob_path(name)?;
        private_access(&self.root, false)?;
        if !path.exists() {
            return Err(IdentityError::MissingLocalState);
        }
        private_access(&path, false)?;
        let file = fs::File::open(path).map_err(|_| IdentityError::PrivateStorage)?;
        let size = file
            .metadata()
            .map_err(|_| IdentityError::PrivateStorage)?
            .len();
        if !(46..=4142).contains(&size) {
            return Err(IdentityError::MissingLocalState);
        }
        let mut record = Zeroizing::new(Vec::new());
        file.take(4143)
            .read_to_end(&mut record)
            .map_err(|_| IdentityError::PrivateStorage)?;
        if record.len() != size as usize || &record[..10] != b"NF-BLOB-1\0" {
            return Err(IdentityError::MissingLocalState);
        }
        let count = u32::from_le_bytes(
            record[10..14]
                .try_into()
                .map_err(|_| IdentityError::MissingLocalState)?,
        ) as usize;
        if count > 4096
            || count + 46 != record.len()
            || Sha256::digest(&record[..count + 14]).as_slice() != &record[count + 14..]
        {
            return Err(IdentityError::MissingLocalState);
        }
        Ok(record[14..14 + count].to_vec())
    }
    pub fn remove_private_blob(&self, name: &str) -> Result<(), IdentityError> {
        let path = self.blob_path(name)?;
        private_access(&self.root, false)?;
        private_access(&path, false)?;
        fs::remove_file(path).map_err(|_| IdentityError::PrivateStorage)
    }
    fn blob_path(&self, name: &str) -> Result<PathBuf, IdentityError> {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || value == b'_' || value == b'-')
        {
            return Err(IdentityError::PrivateStorage);
        }
        Ok(self.root.join(format!("blob-{name}")))
    }
}
