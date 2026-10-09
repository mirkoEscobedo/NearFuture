pub use crate::private_diagnostics::{
    HelperCause, HelperDiagnostic, HelperExitStage, HelperKind, HelperStep, PrivateCause,
    PrivateDiagnostic, PrivateFailure, PrivateOperation, PrivateResult, PrivateStage,
};
use crate::{keys::SecretSeed, model::*};
use nf_contract::identity::{AccountId, DeviceId};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
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
        Self::create_detailed(path, game_save_root).map_err(PrivateFailure::identity_error)
    }
    pub fn open(path: &Path, game_save_root: &Path) -> Result<Self, IdentityError> {
        Self::open_detailed(path, game_save_root).map_err(PrivateFailure::identity_error)
    }
    pub fn create_identity(&self, peer: Vec<u8>) -> Result<LocalIdentity, IdentityError> {
        private_access(&self.root, false)?;
        let (public, account_key, device_key) = crate::keys::generate_identity(peer)?;
        let path = self.root.join("identity-key-v1");
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
        file.write_all(&record)
            .and_then(|()| file.sync_all())
            .map_err(|_| IdentityError::PrivateStorage)?;
        #[cfg(unix)]
        {
            fs::File::open(&self.root)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| IdentityError::PrivateStorage)?;
        }
        Ok(LocalIdentity {
            public,
            account_key,
            device_key,
        })
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
#[cfg(windows)]
pub(crate) fn private_access(path: &Path, initialize: bool) -> Result<(), IdentityError> {
    use std::{
        os::windows::{fs::MetadataExt, process::CommandExt},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let metadata = fs::symlink_metadata(path).map_err(|_| IdentityError::PrivateStorage)?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(IdentityError::PrivateStorage);
    }
    let mut command = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/src/private-acl.ps1"))
        .arg("-PrivatePath")
        .arg(path);
    if initialize {
        command.arg("-Initialize");
    }
    let child = command
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| IdentityError::PrivateStorage)?;
    let mut child = crate::owned_process::OwnedChild::new(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| IdentityError::PrivateStorage)?
        {
            break status;
        }
        if Instant::now() >= deadline {
            return Err(IdentityError::PrivateStorage);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        return Err(IdentityError::PrivateStorage);
    }
    Ok(())
}
#[cfg(unix)]
pub(crate) fn private_access(path: &Path, initialize: bool) -> Result<(), IdentityError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let metadata = fs::symlink_metadata(path).map_err(|_| IdentityError::PrivateStorage)?;
    if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(IdentityError::PrivateStorage);
    }
    if initialize {
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if metadata.is_dir() { 0o700 } else { 0o600 }),
        )
        .map_err(|_| IdentityError::PrivateStorage)?;
    }
    let checked = fs::symlink_metadata(path).map_err(|_| IdentityError::PrivateStorage)?;
    let owner = fs::metadata("/proc/self")
        .map_err(|_| IdentityError::PrivateStorage)?
        .uid();
    if checked.uid() != owner
        || checked.mode() & 0o777 != if checked.is_dir() { 0o700 } else { 0o600 }
    {
        return Err(IdentityError::PrivateStorage);
    }
    Ok(())
}
#[cfg(not(any(windows, unix)))]
pub(crate) fn private_access(_path: &Path, _initialize: bool) -> Result<(), IdentityError> {
    Err(IdentityError::PrivateStorage)
}

pub use crate::protected_blob_scope::ProtectedBlobScope;
