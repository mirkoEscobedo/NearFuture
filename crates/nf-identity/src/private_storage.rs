use crate::{keys::SecretSeed, model::*};
use nf_contract::identity::{AccountId, DeviceId};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

#[path = "private_storage_diagnostics.rs"]
mod diagnostics;
pub use diagnostics::{
    HelperCleanup, HelperObservation, PrivateStorageFailure, PrivateStorageReason,
    PrivateStorageStage, StorageIoFailure,
};

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
        Self::create_diagnosed(path, game_save_root).map_err(PrivateStorageFailure::legacy)
    }
    pub fn open(path: &Path, game_save_root: &Path) -> Result<Self, IdentityError> {
        Self::open_diagnosed(path, game_save_root).map_err(PrivateStorageFailure::legacy)
    }
    /// Same create operation with closed failure context; never returns target or helper text.
    pub fn create_diagnosed(
        path: &Path,
        game_save_root: &Path,
    ) -> Result<Self, PrivateStorageFailure> {
        fs::create_dir(path).map_err(|error| {
            PrivateStorageFailure::with_io(
                PrivateStorageStage::CreateDirectory,
                PrivateStorageReason::Io,
                &error,
            )
        })?;
        private_access_diagnosed(path, true)?;
        Self::open_diagnosed(path, game_save_root)
    }
    /// Same open operation with closed failure context and unchanged ACL policy.
    pub fn open_diagnosed(
        path: &Path,
        game_save_root: &Path,
    ) -> Result<Self, PrivateStorageFailure> {
        private_access_diagnosed(path, false)?;
        let root = fs::canonicalize(path).map_err(|error| {
            PrivateStorageFailure::with_io(
                PrivateStorageStage::CanonicalizeVault,
                PrivateStorageReason::Io,
                &error,
            )
        })?;
        let saves = fs::canonicalize(game_save_root).map_err(|error| {
            PrivateStorageFailure::with_io(
                PrivateStorageStage::CanonicalizeSaves,
                PrivateStorageReason::Io,
                &error,
            )
        })?;
        if root.starts_with(&saves) {
            return Err(PrivateStorageFailure::new(
                PrivateStorageStage::Placement,
                PrivateStorageReason::InsideSaves,
            ));
        }
        if !root.is_dir() {
            return Err(PrivateStorageFailure::new(
                PrivateStorageStage::Placement,
                PrivateStorageReason::DirectoryCheck,
            ));
        }
        Ok(Self { root })
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
pub(crate) fn private_access(path: &Path, initialize: bool) -> Result<(), IdentityError> {
    private_access_diagnosed(path, initialize).map_err(PrivateStorageFailure::legacy)
}
fn acl_stage(initialize: bool) -> PrivateStorageStage {
    if initialize {
        PrivateStorageStage::InitializeAcl
    } else {
        PrivateStorageStage::VerifyAcl
    }
}

#[cfg(windows)]
fn private_access_diagnosed(path: &Path, initialize: bool) -> Result<(), PrivateStorageFailure> {
    use std::{
        os::windows::{fs::MetadataExt, process::CommandExt},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let stage = acl_stage(initialize);
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        PrivateStorageFailure::with_io(stage, PrivateStorageReason::Metadata, &error)
    })?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(PrivateStorageFailure::new(
            stage,
            PrivateStorageReason::Reparse,
        ));
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
    let started = Instant::now();
    let spawned = command
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let spawn_micros = started.elapsed().as_micros();
    let child = match spawned {
        Ok(child) => child,
        Err(error) => {
            let mut failure =
                PrivateStorageFailure::with_io(stage, PrivateStorageReason::Spawn, &error);
            failure.helper = Some(HelperObservation {
                spawn_micros,
                poll_micros: None,
                exit_code: None,
                cleanup: None,
            });
            return Err(failure);
        }
    };
    let mut child = crate::owned_process::OwnedChild::new(child);
    // Preserve the original post-spawn deadline and wait-before-deadline ordering.
    let poll_started = Instant::now();
    let deadline = poll_started + Duration::from_secs(5);
    let refusal = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let mut failure = PrivateStorageFailure::new(stage, PrivateStorageReason::Exit);
                failure.helper = Some(HelperObservation {
                    spawn_micros,
                    poll_micros: Some(poll_started.elapsed().as_micros()),
                    exit_code: status.code(),
                    cleanup: None,
                });
                return Err(failure);
            }
            Ok(None) => {}
            Err(error) => {
                break PrivateStorageFailure::with_io(stage, PrivateStorageReason::Wait, &error);
            }
        }
        if Instant::now() >= deadline {
            break PrivateStorageFailure::new(stage, PrivateStorageReason::Deadline);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let poll_micros = poll_started.elapsed().as_micros();
    let cleanup = child.cleanup().map(|(killed, waited)| HelperCleanup {
        kill_error: killed.err().as_ref().map(StorageIoFailure::from),
        wait_result: waited
            .map(|status| status.code())
            .map_err(|error| StorageIoFailure::from(&error)),
    });
    let mut refusal = refusal;
    refusal.helper = Some(HelperObservation {
        spawn_micros,
        poll_micros: Some(poll_micros),
        exit_code: None,
        cleanup,
    });
    Err(refusal)
}
#[cfg(unix)]
fn private_access_diagnosed(path: &Path, initialize: bool) -> Result<(), PrivateStorageFailure> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let stage = acl_stage(initialize);
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        PrivateStorageFailure::with_io(stage, PrivateStorageReason::Metadata, &error)
    })?;
    if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(PrivateStorageFailure::new(
            stage,
            PrivateStorageReason::UnsafeType,
        ));
    }
    if initialize {
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if metadata.is_dir() { 0o700 } else { 0o600 }),
        )
        .map_err(|error| {
            PrivateStorageFailure::with_io(stage, PrivateStorageReason::Permissions, &error)
        })?;
    }
    let checked = fs::symlink_metadata(path).map_err(|error| {
        PrivateStorageFailure::with_io(stage, PrivateStorageReason::Metadata, &error)
    })?;
    let owner = fs::metadata("/proc/self")
        .map_err(|error| {
            PrivateStorageFailure::with_io(stage, PrivateStorageReason::Metadata, &error)
        })?
        .uid();
    if checked.uid() != owner
        || checked.mode() & 0o777 != if checked.is_dir() { 0o700 } else { 0o600 }
    {
        return Err(PrivateStorageFailure::new(
            stage,
            PrivateStorageReason::Permissions,
        ));
    }
    Ok(())
}
#[cfg(not(any(windows, unix)))]
fn private_access_diagnosed(_path: &Path, initialize: bool) -> Result<(), PrivateStorageFailure> {
    Err(PrivateStorageFailure::new(
        acl_stage(initialize),
        PrivateStorageReason::UnsupportedPlatform,
    ))
}
