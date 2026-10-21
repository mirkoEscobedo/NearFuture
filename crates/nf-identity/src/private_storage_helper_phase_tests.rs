//! Real PS5.1 qualification. Selected separately from the pure child_phase filter.
use super::{ChildPhaseObservation, HelperPhase, PrivateStorageReason, private_access_diagnosed};
use crate::{model::IdentityError, owned_process::OwnedChild};
use std::{
    fs,
    os::windows::{fs::MetadataExt, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct NativeFixture {
    root: PathBuf,
    parent: PathBuf,
}
impl NativeFixture {
    fn new() -> Self {
        let temp = std::env::temp_dir();
        let parent = temp.canonicalize().expect("ordinary fixture parent");
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).unwrap();
        let nonce = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let root = temp.join(format!("nf-acl-native-phase-{nonce}"));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("vault")).unwrap();
        fs::create_dir(root.join("saves")).unwrap();
        Self { root, parent }
    }
}
impl Drop for NativeFixture {
    fn drop(&mut self) {
        let ordinary = fs::symlink_metadata(&self.root)
            .is_ok_and(|value| value.is_dir() && value.file_attributes() & 0x400 == 0);
        let known_parent = self
            .root
            .parent()
            .and_then(|path| path.canonicalize().ok())
            .is_some_and(|path| path == self.parent);
        let known_name = self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("nf-acl-native-phase-") && name.len() == 52);
        if ordinary && known_parent && known_name {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

#[test]
fn real_helper_phase_contract_refuses_then_initializes_with_closed_progress() {
    let _serial = crate::owned_process::ACL_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fixture = NativeFixture::new();
    let vault = fixture.root.join("vault");
    let saves = fixture.root.join("saves");

    // Production path, unchanged inherited ACL: refusal retains the real closed child trace.
    let refused = private_access_diagnosed(&vault, false, Some(&saves))
        .expect_err("fresh inherited ACL must not be admitted as protected private storage");
    eprintln!("helper_phase_native_refusal={refused:?}");
    assert_eq!(refused.reason, PrivateStorageReason::Exit);
    let helper = refused.helper.expect("actual owned helper observation");
    assert_eq!(helper.exit_code, Some(2));
    assert_eq!(
        helper.cleanup, None,
        "natural exit is reaped before any cleanup request"
    );
    assert_eq!(
        helper.child_phase,
        ChildPhaseObservation::Observed {
            last: HelperPhase::Refused,
            records: 5,
        }
    );
    assert_eq!(refused.legacy(), IdentityError::PrivateStorage);

    // Same pinned helper/arguments and existing OwnedChild, now initialize and inspect its
    // parent-created sidecar after actual natural terminal. No replacement helper/framework.
    let mut phases = super::child_phase::PhaseSidecar::prepare(&vault, Some(&saves))
        .expect("ordinary outside-roots phase file");
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
        .arg(&vault)
        .arg("-Initialize")
        .arg("-PhasePath")
        .arg(phases.path())
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = command.spawn().expect("owned PS5.1 helper spawn");
    let pid = child.id();
    let mut child = OwnedChild::new(child);
    let started = Instant::now();
    let deadline = started + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("owned child wait") {
            break status;
        }
        if Instant::now() >= deadline {
            let cleanup = child.cleanup();
            let confirmed = cleanup.as_ref().is_some_and(|(_, waited)| waited.is_ok());
            let progress = phases.observe_after_terminal(confirmed);
            panic!("helper exceeded unchanged post-spawn bound; closed_progress={progress:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let progress = phases.observe_after_terminal(true);
    assert_eq!(status.code(), Some(0), "closed_progress={progress:?}");
    assert_eq!(
        progress,
        ChildPhaseObservation::Observed {
            last: HelperPhase::PolicyVerified,
            records: 7,
        }
    );
    eprintln!(
        "helper_phase_native_refusal_exit=2 progress=Refused records=5; initialize_pid={pid} natural_exit=0 progress=PolicyVerified records=7; poll_micros={}",
        started.elapsed().as_micros()
    );
}
