use crate::{model::IdentityError, owned_process::OwnedChild};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const INPUT_LIMIT: usize = 272384;
pub(crate) fn check(
    paths: &[&Path],
    stage: crate::private_diagnostics::PrivateStage,
) -> crate::private_diagnostics::PrivateResult<()> {
    if paths.is_empty() || paths.len() > 65 {
        return Err(crate::private_diagnostics::PrivateFailure::legacy(
            IdentityError::Limit,
        ));
    }
    let mut input = Vec::with_capacity(INPUT_LIMIT);
    input.extend_from_slice(format!("{}\n", paths.len()).as_bytes());
    for path in paths {
        let text = path
            .to_str()
            .ok_or(crate::private_diagnostics::PrivateFailure::legacy(
                IdentityError::PrivateStorage,
            ))?;
        if text.is_empty()
            || text.len() > 4096
            || text.bytes().any(|v| matches!(v, 0 | b'\r' | b'\n'))
            || input.len() + text.len() + 1 > INPUT_LIMIT
        {
            return Err(crate::private_diagnostics::PrivateFailure::legacy(
                IdentityError::Limit,
            ));
        }
        input.extend_from_slice(text.as_bytes());
        input.push(b'\n');
    }
    run_detailed(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/private-acl-batch.ps1"
        )),
        input,
    )
    .map_err(|helper| {
        crate::private_diagnostics::PrivateFailure::with_cause(
            crate::private_diagnostics::PrivateOperation::BlobScan,
            stage,
            crate::private_diagnostics::PrivateCause::Helper(helper),
        )
    })
}
use crate::private_diagnostics::helper_exit_stage as exit_stage;
use crate::private_diagnostics::{HelperCause, HelperDiagnostic, HelperKind, HelperStep};
fn failed(step: HelperStep, cause: HelperCause) -> HelperDiagnostic {
    HelperDiagnostic::new(HelperKind::BatchAcl, step, cause)
}
fn io(step: HelperStep, error: std::io::Error) -> HelperDiagnostic {
    HelperDiagnostic::io(HelperKind::BatchAcl, step, error)
}
#[cfg(test)]
fn run(script: &Path, input: Vec<u8>) -> Result<(), IdentityError> {
    run_detailed(script, input).map_err(|_| IdentityError::PrivateStorage)
}
fn run_detailed(script: &Path, input: Vec<u8>) -> Result<(), HelperDiagnostic> {
    std::thread::scope(|scope| {
        use std::os::windows::process::CommandExt;
        let started = Instant::now();
        let mut child = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| io(HelperStep::Spawn, error))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        // Installed before any fallible extraction/thread work; closure locals drop before scope joins.
        let mut child = OwnedChild::new(child);
        let mut stdin = stdin.ok_or(failed(HelperStep::InputPipe, HelperCause::MissingPipe))?;
        let mut stdout = stdout.ok_or(failed(HelperStep::OutputPipe, HelperCause::MissingPipe))?;
        let mut stderr = stderr.ok_or(failed(HelperStep::ErrorPipe, HelperCause::MissingPipe))?;
        let writer = scope.spawn(move || stdin.write_all(&input));
        let output = scope.spawn(move || stdout.read(&mut [0u8; 1]));
        let errors = scope.spawn(move || stderr.read(&mut [0u8; 1]));
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| io(HelperStep::Wait, error))?
            {
                if !status.success() {
                    return Err(failed(
                        HelperStep::Exit,
                        HelperCause::ChildExit {
                            code: status.code(),
                            stage: exit_stage(HelperKind::BatchAcl, status.code()),
                        },
                    ));
                }
                break;
            }
            if started.elapsed() >= Duration::from_secs(5) {
                return Err(failed(
                    HelperStep::Wait,
                    HelperCause::Timeout {
                        elapsed_ms: u64::try_from(started.elapsed().as_millis())
                            .unwrap_or(u64::MAX),
                    },
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        writer
            .join()
            .map_err(|_| failed(HelperStep::InputJoin, HelperCause::ThreadPanic))?
            .map_err(|error| io(HelperStep::InputWrite, error))?;
        let out = output
            .join()
            .map_err(|_| failed(HelperStep::OutputJoin, HelperCause::ThreadPanic))?
            .map_err(|error| io(HelperStep::OutputRead, error))?;
        let err = errors
            .join()
            .map_err(|_| failed(HelperStep::ErrorJoin, HelperCause::ThreadPanic))?
            .map_err(|error| io(HelperStep::ErrorRead, error))?;
        if out != 0 || err != 0 {
            return Err(failed(
                HelperStep::OutputPolicy,
                HelperCause::UnexpectedOutput {
                    stdout: out != 0,
                    stderr: err != 0,
                },
            ));
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "private_blob_acl_tests.rs"]
mod tests;
