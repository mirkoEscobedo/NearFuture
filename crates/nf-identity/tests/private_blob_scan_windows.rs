#![cfg(windows)]
#[path = "support/blob_scan.rs"]
mod support;
use nf_identity::model::IdentityError;
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
#[test]
fn middle_foreign_acl_cannot_be_hidden_by_later_valid_paths() {
    let (root, vault) = support::fixture();
    for name in ["receipt-r0-g0", "receipt-r3-g3", "receipt-r7-g7"] {
        vault.create_private_blob(name, &[7]).unwrap();
    }
    run_owned(
        Command::new("C:/Windows/System32/icacls.exe")
            .arg(root.join("private/blob-receipt-r3-g3"))
            .args(["/grant", "*S-1-1-0:R"]),
    );
    let names = support::names();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    assert_eq!(
        vault.scan_optional_private_blobs(&names, |_, _| panic!("foreign ACL admitted")),
        Err(IdentityError::PrivateStorage)
    );
}
#[test]
fn actual_reparse_tail_is_not_an_empty_inventory() {
    use std::os::windows::fs::MetadataExt;
    let (root, vault) = support::fixture();
    let target = root.join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("marker"), b"public marker").unwrap();
    let junction = root.join("private/blob-receipt-r7-g7");
    let script = root.join("junction.ps1");
    fs::write(&script,"param([string]$Junction,[string]$Target)\n$ErrorActionPreference='Stop'\nNew-Item -ItemType Junction -Path $Junction -Target $Target | Out-Null\n").unwrap();
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
    let names = support::names();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    assert_eq!(
        vault.scan_optional_private_blobs(&names, |_, _| panic!("reparse admitted")),
        Err(IdentityError::PrivateStorage)
    );
    fs::remove_dir(&junction).unwrap();
    assert_eq!(fs::read(target.join("marker")).unwrap(), b"public marker");
}
#[test]
fn final_root_permission_change_refuses_provisional_empty_observations() {
    let (root, vault) = support::fixture();
    let names = support::names();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    let mut callbacks = 0;
    assert_eq!(
        vault.scan_optional_private_blobs(&names, |index, _| {
            callbacks += 1;
            if index == 63 {
                // Trusted fixture fault injection; production visitor must not mutate vault policy.
                run_owned(
                    Command::new("C:/Windows/System32/icacls.exe")
                        .arg(root.join("private"))
                        .args(["/grant", "*S-1-1-0:R"]),
                );
            }
            Ok(())
        }),
        Err(IdentityError::PrivateStorage)
    );
    assert_eq!(callbacks, 64);
}
