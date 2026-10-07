use super::*;
use std::{fs, path::PathBuf};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        fs::create_dir_all(&base).unwrap();
        let name: String = crate::keys::random_id()
            .unwrap()
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect();
        let root = base.join(format!("batch-io-{name}"));
        assert!(root.starts_with(&base));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn script(&self, text: &str) -> PathBuf {
        let path = self.0.join("fixture.ps1");
        fs::write(&path, text).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn early_helper_exit_unblocks_owned_full_pipe_writer() {
    let root = Scratch::new();
    assert_eq!(run(&root.script("exit 0"), vec![]), Ok(()));
    let start = Instant::now();
    assert_eq!(
        run(&root.script("exit 2"), vec![b'x'; INPUT_LIMIT]),
        Err(IdentityError::PrivateStorage)
    );
    assert!(start.elapsed() < Duration::from_secs(10));
}
#[test]
fn unexpected_helper_output_is_a_refusal() {
    let root = Scratch::new();
    assert_eq!(
        run(
            &root.script("Write-Output 'public fixture noise'; exit 0"),
            vec![]
        ),
        Err(IdentityError::PrivateStorage)
    );
}
#[test]
fn actual_five_second_timeout_kills_helper_before_scoped_pipe_join() {
    let root = Scratch::new();
    let start = Instant::now();
    assert_eq!(
        run(
            &root.script("Start-Sleep -Seconds 30"),
            vec![b'x'; INPUT_LIMIT]
        ),
        Err(IdentityError::PrivateStorage)
    );
    assert!(start.elapsed() >= Duration::from_secs(5));
    assert!(start.elapsed() < Duration::from_secs(10));
}
#[test]
fn actual_sidecar_rejects_malformed_count_trailing_and_oversized_lines() {
    let script = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/private-acl-batch.ps1"
    ));
    for input in [
        b"0\n".to_vec(),
        b"66\n".to_vec(),
        b"1\n\n".to_vec(),
        b"1\npath\ntrailing\n".to_vec(),
        [
            b"1\n".as_slice(),
            vec![b'x'; 4097].as_slice(),
            b"\n".as_slice(),
        ]
        .concat(),
    ] {
        assert_eq!(run(script, input), Err(IdentityError::PrivateStorage));
    }
}

#[test]
fn detailed_child_exit_retains_closed_stage_without_output() {
    use crate::private_diagnostics::{HelperCause, HelperExitStage, HelperKind, HelperStep};
    let root = Scratch::new();
    let failure = run_detailed(&root.script("exit 11"), vec![b'x'; INPUT_LIMIT]).unwrap_err();
    assert_eq!(failure.kind(), HelperKind::BatchAcl);
    assert_eq!(failure.step(), HelperStep::Exit);
    assert_eq!(
        failure.cause(),
        HelperCause::ChildExit {
            code: Some(11),
            stage: Some(HelperExitStage::GetItem)
        }
    );
}
#[test]
fn detailed_timeout_retains_wait_stage_and_bounded_elapsed_time() {
    use crate::private_diagnostics::{HelperCause, HelperKind, HelperStep};
    let root = Scratch::new();
    let started = Instant::now();
    let failure = run_detailed(
        &root.script("Start-Sleep -Seconds 30"),
        vec![b'x'; INPUT_LIMIT],
    )
    .unwrap_err();
    assert_eq!(failure.kind(), HelperKind::BatchAcl);
    assert_eq!(failure.step(), HelperStep::Wait);
    match failure.cause() {
        HelperCause::Timeout { elapsed_ms } => assert!((5000..10000).contains(&elapsed_ms)),
        other => panic!("unexpected passive cause: {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(10));
}
#[test]
fn detailed_forbidden_output_records_presence_only() {
    use crate::private_diagnostics::{HelperCause, HelperStep};
    let root = Scratch::new();
    for (script, stdout, stderr) in [
        ("[Console]::Out.Write('fixture'); exit 0", true, false),
        ("[Console]::Error.Write('fixture'); exit 0", false, true),
    ] {
        let failure = run_detailed(&root.script(script), vec![]).unwrap_err();
        assert_eq!(failure.step(), HelperStep::OutputPolicy);
        assert_eq!(
            failure.cause(),
            HelperCause::UnexpectedOutput { stdout, stderr }
        );
    }
}
#[test]
fn actual_sidecar_returns_closed_phase_codes_and_preserves_single_phase() {
    use crate::private_diagnostics::{HelperCause, HelperExitStage};
    let script = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/private-acl-batch.ps1"
    ));
    for (input, code, stage) in [
        (b"0\n".as_slice(), 21, HelperExitStage::BatchCount),
        (b"1\n\n".as_slice(), 22, HelperExitStage::BatchPath),
        (
            b"1\nfixture\ntrailing\n".as_slice(),
            23,
            HelperExitStage::BatchTrailing,
        ),
        (b"1\n".as_slice(), 20, HelperExitStage::BatchRead),
        (
            b"1\nnot-a-real-private-fixture\n".as_slice(),
            11,
            HelperExitStage::GetItem,
        ),
    ] {
        let failure = run_detailed(script, input.to_vec()).unwrap_err();
        assert_eq!(
            failure.cause(),
            HelperCause::ChildExit {
                code: Some(code),
                stage: Some(stage)
            }
        );
    }
}
#[test]
fn detailed_input_write_failure_retains_io_class_only() {
    use crate::private_diagnostics::{HelperCause, HelperStep};
    let root = Scratch::new();
    let failure = run_detailed(&root.script("exit 0"), vec![b'x'; INPUT_LIMIT]).unwrap_err();
    assert_eq!(failure.step(), HelperStep::InputWrite);
    match failure.cause() {
        HelperCause::Io { kind, .. } => assert_eq!(kind, std::io::ErrorKind::BrokenPipe),
        other => panic!("unexpected passive cause: {other:?}"),
    }
}
