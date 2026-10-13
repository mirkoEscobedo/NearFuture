#![cfg(windows)]
// The ACL boundary must not load cmdlet modules inside its fixed helper budget.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn helper(path: &Path, initialize: bool) -> ExitStatus {
    use std::os::windows::process::CommandExt;
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/src/private-acl.ps1");
    let command = format!(
        "$PSModuleAutoLoadingPreference='None'; function Import-Module {{ throw 'MODULE_LOADING_FORBIDDEN' }}; & '{}' -PrivatePath '{}' {}; exit $LASTEXITCODE",
        script.replace('\'', "''"),
        path.to_str().unwrap().replace('\'', "''"),
        if initialize { "-Initialize" } else { "" },
    );
    let mut child = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
        ])
        .arg(command)
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("owned helper wait failed: {:?}", error.kind());
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("owned helper exceeded the original five-second polling limit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn module_free_acl_admits_literal_directory_and_file_then_refuses_relaxed_file() {
    let root = std::env::temp_dir().join(format!("nf-acl-module-free-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let directory = scratch.0.join("owner ' [literal] directory");
    fs::create_dir(&directory).unwrap();
    let file = directory.join("owner ' [literal] file");
    fs::write(&file, b"owned synthetic fixture").unwrap();
    for path in [&directory, &file] {
        assert!(
            helper(path, true).success(),
            "module-free initialization refused"
        );
        assert!(
            helper(path, false).success(),
            "module-free verification refused"
        );
    }
    let report = Command::new("C:/Windows/System32/icacls.exe")
        .arg(&file)
        .output()
        .unwrap();
    assert!(report.status.success());
    let report = String::from_utf8(report.stdout).unwrap();
    assert!(!report.contains("(I)") && !report.contains("S-1-1-0") && report.contains("(F)"));
    let changed = Command::new("C:/Windows/System32/icacls.exe")
        .arg(&file)
        .args(["/grant", "*S-1-1-0:R"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(changed.success());
    assert_eq!(helper(&file, false).code(), Some(2));
    assert_eq!(fs::read(&file).unwrap(), b"owned synthetic fixture");
}
