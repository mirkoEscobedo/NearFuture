use nf_identity::{keys::random_id, model::IdentityError, private_storage::PrivateVault};
use std::{fs, path::PathBuf};
struct Disposable(PathBuf);
impl std::ops::Deref for Disposable {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0
    }
}
impl AsRef<std::path::Path> for Disposable {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Disposable {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn disposable() -> Disposable {
    let name: String = random_id()
        .unwrap()
        .iter()
        .map(|value| format!("{value:02x}"))
        .collect();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.tmp")
        .join(format!("identity-{name}"));
    fs::create_dir_all(&path).unwrap();
    Disposable(path)
}
#[test]
fn private_keys_stay_outside_save_copies_and_missing_keys_never_recreate_identity() {
    let root = disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    fs::write(saves.join("campaign.save"), b"synthetic disposable save").unwrap();
    let private = root.join("owner private ' space");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    let identity = vault.create_identity(vec![4, 5]).unwrap();
    assert_eq!(
        format!("{:?}", identity.account_key),
        "SecretSeed(REDACTED)"
    );
    let reopened = PrivateVault::open(&private, &saves).unwrap();
    assert_eq!(
        reopened.load_identity(vec![4, 5]).unwrap().public,
        identity.public
    );
    assert_eq!(
        vault.create_identity(vec![4, 5]).err(),
        Some(IdentityError::PrivateStorage)
    );
    let copied = root.join("copied save");
    fs::create_dir(&copied).unwrap();
    fs::copy(saves.join("campaign.save"), copied.join("campaign.save")).unwrap();
    assert_eq!(fs::read_dir(&copied).unwrap().count(), 1);
    assert!(!copied.join("identity-key-v1").exists());
    assert_eq!(
        PrivateVault::create(&saves.join("forbidden private"), &saves).err(),
        Some(IdentityError::PrivateStorage)
    );
    use std::io::{Read, Seek, SeekFrom, Write};
    let mut damaged = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(private.join("identity-key-v1"))
        .unwrap();
    damaged.seek(SeekFrom::Start(60)).unwrap();
    let mut byte = [0u8; 1];
    damaged.read_exact(&mut byte).unwrap();
    byte[0] ^= 1;
    damaged.seek(SeekFrom::Start(60)).unwrap();
    damaged.write_all(&byte).unwrap();
    damaged.sync_all().unwrap();
    drop(damaged);
    assert_eq!(
        reopened.load_identity(vec![4, 5]).err(),
        Some(IdentityError::MissingLocalState)
    );
    fs::remove_file(private.join("identity-key-v1")).unwrap();
    assert_eq!(
        reopened.load_identity(vec![4, 5]).err(),
        Some(IdentityError::MissingLocalState)
    );
    assert!(!private.join("identity-key-v1").exists());
    fs::remove_dir_all(root).unwrap();
}
#[cfg(windows)]
#[test]
fn windows_acl_is_protected_owner_only_and_relaxing_it_blocks_private_reads() {
    use std::process::Command;
    let root = disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let private = root.join("private");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    vault.create_identity(vec![1]).unwrap();
    // Independent ACL inspection via icacls. No key material is read or logged.
    let output = Command::new("C:/Windows/System32/icacls.exe")
        .arg(private.join("identity-key-v1"))
        .output()
        .unwrap();
    assert!(output.status.success());
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(!report.contains("(I)") && !report.contains("S-1-1-0") && report.contains("(F)"));
    let status = Command::new("C:/Windows/System32/icacls.exe")
        .arg(private.join("identity-key-v1"))
        .args(["/grant", "*S-1-1-0:R"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        vault.load_identity(vec![1]).err(),
        Some(IdentityError::PrivateStorage)
    );
    fs::remove_dir_all(root).unwrap();
}
#[cfg(unix)]
#[test]
fn unix_modes_reject_group_readable_private_keys() {
    use std::os::unix::fs::PermissionsExt;
    let root = disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let private = root.join("private");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    vault.create_identity(vec![1]).unwrap();
    assert_eq!(
        fs::metadata(&private).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let file = private.join("identity-key-v1");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(file, fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(
        vault.load_identity(vec![1]).err(),
        Some(IdentityError::PrivateStorage)
    );
    fs::remove_dir_all(root).unwrap();
}
#[cfg(windows)]
#[test]
fn windows_acl_helper_accepts_literal_paths_without_shell_interpolation() {
    use std::process::Command;
    let root = disposable();
    let private = root.join("literal private");
    fs::create_dir(&private).unwrap();
    let output = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
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
        .arg(&private)
        .arg("-Initialize")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ACL helper status {:?}; public setup diagnostic: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn staged_rotation_keys_reload_only_when_persisted_public_identity_selects_them() {
    let root = disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let private = root.join("private");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    let original = vault.create_identity(vec![1]).unwrap();
    let replacement = nf_identity::keys::SecretSeed::generate().unwrap();
    let mut selected = original.public.clone();
    selected.device_key = replacement.public_key();
    selected.peer = vec![2];
    assert_eq!(
        vault.load_membership_keys(&selected).err(),
        Some(IdentityError::MissingLocalState)
    );
    vault.stage_rotation_key(&replacement).unwrap();
    assert_eq!(
        vault.load_identity(vec![1]).unwrap().public,
        original.public
    );
    let reopened = PrivateVault::open(&private, &saves).unwrap();
    let loaded = reopened.load_membership_keys(&selected).unwrap();
    assert_eq!(loaded.device_key.public_key(), replacement.public_key());
    assert_eq!(loaded.account_key.public_key(), original.public.account_key);
    assert_eq!(loaded.device_key.sign(&[8; 32]), replacement.sign(&[8; 32]));
    selected.account =
        nf_contract::identity::AccountId::from_bytes(nf_identity::keys::random_id().unwrap());
    assert_eq!(
        reopened.load_membership_keys(&selected).err(),
        Some(IdentityError::MissingLocalState)
    );
}
