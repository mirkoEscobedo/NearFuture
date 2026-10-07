use crate::{model::IdentityError, owned_process::OwnedChild};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const INPUT_LIMIT: usize = 272384;
/// Closed diagnostics: never retain private paths, helper output, or OS error text.
#[derive(Debug, PartialEq, Eq)]
enum AclHelperFailure {
    Spawn,
    StdinPipe,
    StdoutPipe,
    StderrPipe,
    Wait,
    Deadline,
    Exit,
    InputJoin,
    InputIo,
    StdoutJoin,
    StdoutIo,
    StderrJoin,
    StderrIo,
    UnexpectedStdout,
    UnexpectedStderr,
}
pub(crate) fn check(paths: &[&Path]) -> Result<(), IdentityError> {
    if paths.is_empty() || paths.len() > 65 {
        return Err(IdentityError::Limit);
    }
    let mut input = Vec::with_capacity(INPUT_LIMIT);
    input.extend_from_slice(format!("{}\n", paths.len()).as_bytes());
    for path in paths {
        let text = path.to_str().ok_or(IdentityError::PrivateStorage)?;
        if text.is_empty()
            || text.len() > 4096
            || text.bytes().any(|v| matches!(v, 0 | b'\r' | b'\n'))
            || input.len() + text.len() + 1 > INPUT_LIMIT
        {
            return Err(IdentityError::Limit);
        }
        input.extend_from_slice(text.as_bytes());
        input.push(b'\n');
    }
    run(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/private-acl-batch.ps1"
        )),
        input,
    )
}
fn run(script: &Path, input: Vec<u8>) -> Result<(), IdentityError> {
    run_with_reason(script, input).map_err(|_| IdentityError::PrivateStorage)
}
fn run_with_reason(script: &Path, input: Vec<u8>) -> Result<(), AclHelperFailure> {
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
            .map_err(|_| AclHelperFailure::Spawn)?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        // Installed before any fallible extraction/thread work; closure locals drop before scope joins.
        let mut child = OwnedChild::new(child);
        let mut stdin = stdin.ok_or(AclHelperFailure::StdinPipe)?;
        let mut stdout = stdout.ok_or(AclHelperFailure::StdoutPipe)?;
        let mut stderr = stderr.ok_or(AclHelperFailure::StderrPipe)?;
        let writer = scope.spawn(move || stdin.write_all(&input));
        let output = scope.spawn(move || stdout.read(&mut [0u8; 1]));
        let errors = scope.spawn(move || stderr.read(&mut [0u8; 1]));
        loop {
            if let Some(status) = child.try_wait().map_err(|_| AclHelperFailure::Wait)? {
                if !status.success() {
                    return Err(AclHelperFailure::Exit);
                }
                break;
            }
            if started.elapsed() >= Duration::from_secs(5) {
                return Err(AclHelperFailure::Deadline);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        writer
            .join()
            .map_err(|_| AclHelperFailure::InputJoin)?
            .map_err(|_| AclHelperFailure::InputIo)?;
        let out = output
            .join()
            .map_err(|_| AclHelperFailure::StdoutJoin)?
            .map_err(|_| AclHelperFailure::StdoutIo)?;
        let err = errors
            .join()
            .map_err(|_| AclHelperFailure::StderrJoin)?
            .map_err(|_| AclHelperFailure::StderrIo)?;
        if out != 0 {
            return Err(AclHelperFailure::UnexpectedStdout);
        }
        if err != 0 {
            return Err(AclHelperFailure::UnexpectedStderr);
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "private_blob_acl_tests.rs"]
mod tests;
