//! Real PS5.1 health emitter and owned files; no replacement ACL implementation.
use super::super::child_phase::PhaseSidecar;
use super::super::writer_health::{self, FirstWriterHealth, HealthAdmission};
use super::*;
use std::path::Path;

struct Terminal {
    code: Option<i32>,
    natural: bool,
    poll_micros: u128,
}

fn run_helper(
    script: &Path,
    fixture: &NativeFixture,
    phase: Option<&PhaseSidecar>,
    health: Option<&PhaseSidecar>,
) -> Terminal {
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
        .arg(script)
        .arg("-PrivatePath")
        .arg(fixture.root.join("vault"))
        .arg("-Initialize");
    if let Some(phase) = phase {
        command.arg("-PhasePath").arg(phase.path());
    }
    if let Some(health) = health {
        command.arg("-HealthPath").arg(health.path());
    }
    let child = command
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("owned PS5.1 health helper");
    let pid = child.id();
    let mut child = OwnedChild::new(child);
    let started = Instant::now();
    let deadline = started + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("owned helper wait") {
            eprintln!(
                "helper_health_pid={pid} natural_terminal=true exit_code={:?}",
                status.code()
            );
            return Terminal {
                code: status.code(),
                natural: true,
                poll_micros: started.elapsed().as_micros(),
            };
        }
        if Instant::now() >= deadline {
            let (_, waited) = child.cleanup().expect("owned helper cleanup");
            let status = waited.expect("confirmed owned terminal before health read");
            eprintln!(
                "helper_health_pid={pid} natural_terminal=false cleanup_terminal=true exit_code={:?}",
                status.code()
            );
            return Terminal {
                code: status.code(),
                natural: false,
                poll_micros: started.elapsed().as_micros(),
            };
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn sidecars(fixture: &NativeFixture) -> (PhaseSidecar, PhaseSidecar) {
    let vault = fixture.root.join("vault");
    let saves = fixture.root.join("saves");
    let phase = PhaseSidecar::prepare_captured(&vault, Some(&saves))
        .0
        .expect("primary sidecar");
    let (health, admission) = writer_health::prepare(&vault, Some(&saves), true);
    assert_eq!(admission, HealthAdmission::Prepared);
    let health = health.expect("independently owned health sidecar");
    assert_ne!(phase.path(), health.path());
    (phase, health)
}

fn helper_path() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/private-acl.ps1"))
}

#[test]
fn real_helper_health_distinguishes_completed_and_refused_first_phase_publication() {
    let _serial = crate::owned_process::ACL_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for failed_first in [false, true] {
        let fixture = NativeFixture::new();
        let (mut phase, mut health) = sidecars(&fixture);
        if failed_first {
            fs::write(phase.path(), b"x").unwrap();
        }
        let terminal = run_helper(helper_path(), &fixture, Some(&phase), Some(&health));
        assert!(
            terminal.natural,
            "helper exceeded unchanged five-second bound"
        );
        assert_eq!(
            terminal.code,
            Some(0),
            "health never changes ACL initialization success"
        );
        let actual = writer_health::observe(Some(&mut health), HealthAdmission::Prepared, true);
        assert_eq!(actual.read, PhaseRead::Observed);
        assert_eq!(
            actual.observation,
            if failed_first {
                FirstWriterHealth::FirstFailed
            } else {
                FirstWriterHealth::FirstPublished
            }
        );
        let primary = phase.observe_captured_after_terminal(true);
        if failed_first {
            assert_eq!(
                primary,
                (ChildPhaseObservation::Unknown, PhaseRead::Invalid)
            );
            assert_eq!(fs::read(phase.path()).unwrap(), b"x");
        } else {
            assert_eq!(
                primary,
                (
                    ChildPhaseObservation::Observed {
                        last: HelperPhase::PolicyVerified,
                        records: 7
                    },
                    PhaseRead::Observed
                )
            );
        }
        eprintln!(
            "writer_health_first_failed={failed_first} natural_exit=0 poll_micros={} health={actual:?}",
            terminal.poll_micros
        );
    }
}

#[test]
fn real_helper_suppressed_health_does_not_change_acl_or_primary_phase_success() {
    let _serial = crate::owned_process::ACL_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fixture = NativeFixture::new();
    let (mut phase, mut health) = sidecars(&fixture);
    fs::write(health.path(), b"x").unwrap();
    let terminal = run_helper(helper_path(), &fixture, Some(&phase), Some(&health));
    assert!(terminal.natural);
    assert_eq!(terminal.code, Some(0));
    assert_eq!(
        phase.observe_captured_after_terminal(true),
        (
            ChildPhaseObservation::Observed {
                last: HelperPhase::PolicyVerified,
                records: 7
            },
            PhaseRead::Observed
        )
    );
    let actual = writer_health::observe(Some(&mut health), HealthAdmission::Prepared, true);
    assert_eq!(actual.observation, FirstWriterHealth::Unknown);
    assert_eq!(actual.read, PhaseRead::Invalid);
    assert_eq!(fs::read(health.path()).unwrap(), b"x");
}

#[test]
fn real_helper_without_primary_admission_never_labels_a_skipped_attempt_failed() {
    let _serial = crate::owned_process::ACL_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fixture = NativeFixture::new();
    let (_phase, mut health) = sidecars(&fixture);
    // Deliberately supply health without PhasePath to exercise the helper's guard;
    // the production parent never prepares this combination.
    let terminal = run_helper(helper_path(), &fixture, None, Some(&health));
    assert!(terminal.natural);
    assert_eq!(terminal.code, Some(0));
    let actual = writer_health::observe(Some(&mut health), HealthAdmission::Prepared, true);
    assert_eq!(actual.observation, FirstWriterHealth::Unknown);
    assert_eq!(actual.read, PhaseRead::Empty);
}

#[test]
fn real_health_body_prefix_survives_a_synthetic_blocked_first_open_until_owned_cleanup() {
    let _serial = crate::owned_process::ACL_HELPER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fixture = NativeFixture::new();
    let (mut phase, mut health) = sidecars(&fixture);
    let original = fs::read_to_string(helper_path()).unwrap();
    let boundary = "        $stream = [IO.File]::Open($PhasePath,[IO.FileMode]::Open,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)";
    assert_eq!(original.matches(boundary).count(), 1);
    let blocked = original.replace(
        boundary,
        &format!("        Start-Sleep -Seconds 30\n{boundary}"),
    );
    let script = fixture.root.join("blocked-first-open.ps1");
    fs::write(&script, blocked).unwrap();
    let terminal = run_helper(&script, &fixture, Some(&phase), Some(&health));
    assert!(
        !terminal.natural,
        "synthetic open boundary must wait for original five-second cleanup"
    );
    assert_eq!(
        phase.observe_captured_after_terminal(true),
        (ChildPhaseObservation::Unknown, PhaseRead::Empty)
    );
    let actual = writer_health::observe(Some(&mut health), HealthAdmission::Prepared, true);
    assert_eq!(actual.observation, FirstWriterHealth::BodyEntered);
    assert_eq!(actual.read, PhaseRead::Observed);
}
