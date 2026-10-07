#![cfg(windows)]
#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::{model::IdentityError, private_storage::PrivateVault};
use std::{
    fs,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn run_owned(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    let mut child = OwnedChild(
        command
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "public filesystem fixture setup failed");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "public filesystem fixture setup timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn fixture() -> (temp_dir::Disposable, PrivateVault) {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    (root, vault)
}
#[test]
fn actual_windows_reparse_entry_is_not_an_absent_receipt() {
    use std::os::windows::fs::MetadataExt;
    let (root, vault) = fixture();
    let target = root.join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("marker"), b"public target marker").unwrap();
    let junction = root.join("private/blob-link");
    let script = root.join("junction.ps1");
    fs::write(&script, "param([string]$Junction,[string]$Target)\n$ErrorActionPreference='Stop'\nNew-Item -ItemType Junction -Path $Junction -Target $Target | Out-Null\n").unwrap();
    run_owned(
        Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .arg("-Junction")
            .arg(&junction)
            .arg("-Target")
            .arg(&target),
    );
    assert_ne!(
        fs::symlink_metadata(&junction).unwrap().file_attributes() & 0x400,
        0
    );
    assert_eq!(
        vault.read_optional_private_blob("link"),
        Err(IdentityError::PrivateStorage)
    );
    fs::remove_dir(&junction).unwrap();
    assert_eq!(
        fs::read(target.join("marker")).unwrap(),
        b"public target marker"
    );
}
#[test]
fn a_present_blob_with_relaxed_windows_acl_is_refused() {
    let (root, vault) = fixture();
    vault.create_private_blob("relaxed", &[4]).unwrap();
    run_owned(
        Command::new("C:/Windows/System32/icacls.exe")
            .arg(root.join("private/blob-relaxed"))
            .args(["/grant", "*S-1-1-0:R"]),
    );
    assert_eq!(
        vault.read_optional_private_blob("relaxed"),
        Err(IdentityError::PrivateStorage)
    );
}
