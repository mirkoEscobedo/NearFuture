use crate::{
    keys::SecretSeed,
    model::*,
    private_storage::{LocalIdentity, PrivateVault, private_access},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
};
use zeroize::Zeroizing;

impl PrivateVault {
    /// Stage before membership commit; staging grants no role or active-key transition.
    pub fn stage_rotation_key(&self, seed: &SecretSeed) -> Result<(), IdentityError> {
        private_access(&self.root, false)?;
        let public = seed.public_key();
        let path = self.staged_path(&public);
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
        let mut record = Zeroizing::new(Vec::with_capacity(106));
        record.extend_from_slice(b"NF-SEED-1\0");
        record.extend_from_slice(&public);
        record.extend_from_slice(seed.private_bytes());
        let hash = Sha256::digest(&*record);
        record.extend_from_slice(&hash);
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
    /// `expected` must come from durable, authorized membership, never an incoming unauthenticated record.
    pub fn load_membership_keys(
        &self,
        expected: &PublicIdentity,
    ) -> Result<LocalIdentity, IdentityError> {
        let original = self.load_identity(expected.peer.clone())?;
        if original.public.account != expected.account || original.public.device != expected.device
        {
            return Err(IdentityError::MissingLocalState);
        }
        let account_key = if original.account_key.public_key() == expected.account_key {
            original.account_key
        } else {
            self.load_staged(&expected.account_key)?
        };
        let device_key = if original.device_key.public_key() == expected.device_key {
            original.device_key
        } else {
            self.load_staged(&expected.device_key)?
        };
        Ok(LocalIdentity {
            public: expected.clone(),
            account_key,
            device_key,
        })
    }
    fn staged_path(&self, public: &[u8; 32]) -> std::path::PathBuf {
        let id: String = public.iter().map(|byte| format!("{byte:02x}")).collect();
        self.root.join(format!("staged-key-{id}"))
    }
    fn load_staged(&self, public: &[u8; 32]) -> Result<SecretSeed, IdentityError> {
        let path = self.staged_path(public);
        if !path.exists() {
            return Err(IdentityError::MissingLocalState);
        }
        private_access(&path, false)?;
        let mut file = fs::File::open(&path).map_err(|_| IdentityError::PrivateStorage)?;
        if file
            .metadata()
            .map_err(|_| IdentityError::PrivateStorage)?
            .len()
            != 106
        {
            return Err(IdentityError::MissingLocalState);
        }
        let mut record = Zeroizing::new([0u8; 106]);
        file.read_exact(&mut *record)
            .map_err(|_| IdentityError::MissingLocalState)?;
        if &record[..10] != b"NF-SEED-1\0"
            || &record[10..42] != public
            || Sha256::digest(&record[..74]).as_slice() != &record[74..]
        {
            return Err(IdentityError::MissingLocalState);
        }
        let seed = SecretSeed::from_private_slice(&record[42..74])?;
        if &seed.public_key() != public {
            return Err(IdentityError::MissingLocalState);
        }
        Ok(seed)
    }
}
