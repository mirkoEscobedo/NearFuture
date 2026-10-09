use super::PrivateCause;
#[cfg(windows)]
use super::{HelperCause, HelperDiagnostic, HelperKind, HelperStep};
use std::{fs, path::Path};
#[cfg(windows)]
pub(crate) fn check(path: &Path, initialize: bool) -> Result<(), PrivateCause> {
    use std::os::windows::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        PrivateCause::Helper(HelperDiagnostic::io(
            HelperKind::SingleAcl,
            HelperStep::Metadata,
            error,
        ))
    })?;
    if metadata.file_attributes() & 0x400 != 0 {
        return Err(PrivateCause::Helper(HelperDiagnostic::new(
            HelperKind::SingleAcl,
            HelperStep::Reparse,
            HelperCause::Refused,
        )));
    }
    run(
        path,
        initialize,
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/private-acl.ps1")),
    )
    .map_err(PrivateCause::Helper)
}
#[cfg(windows)]
fn run(path: &Path, initialize: bool, script: &Path) -> Result<(), HelperDiagnostic> {
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
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
        .arg(path);
    if initialize {
        command.arg("-Initialize");
    }
    let child = command
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| HelperDiagnostic::io(HelperKind::SingleAcl, HelperStep::Spawn, error))?;
    let mut child = crate::owned_process::OwnedChild::new(child);
    let started = Instant::now();
    let deadline = started + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| HelperDiagnostic::io(HelperKind::SingleAcl, HelperStep::Wait, error))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            return Err(HelperDiagnostic::new(
                HelperKind::SingleAcl,
                HelperStep::Wait,
                HelperCause::Timeout {
                    elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                },
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        return Err(HelperDiagnostic::new(
            HelperKind::SingleAcl,
            HelperStep::Exit,
            HelperCause::ChildExit {
                code: status.code(),
                stage: super::helper_exit_stage(HelperKind::SingleAcl, status.code()),
            },
        ));
    }
    Ok(())
}
#[cfg(unix)]
pub(crate) fn check(path: &Path, initialize: bool) -> Result<(), PrivateCause> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    fn io(error: std::io::Error) -> PrivateCause {
        PrivateCause::Io {
            kind: error.kind(),
            os_code: error.raw_os_error(),
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(io)?;
    if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(PrivateCause::Refused);
    }
    if initialize {
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if metadata.is_dir() { 0o700 } else { 0o600 }),
        )
        .map_err(io)?;
    }
    let checked = fs::symlink_metadata(path).map_err(io)?;
    let owner = fs::metadata("/proc/self").map_err(io)?.uid();
    if checked.uid() != owner
        || checked.mode() & 0o777 != if checked.is_dir() { 0o700 } else { 0o600 }
    {
        return Err(PrivateCause::Refused);
    }
    Ok(())
}
#[cfg(not(any(windows, unix)))]
pub(crate) fn check(_path: &Path, _initialize: bool) -> Result<(), PrivateCause> {
    Err(PrivateCause::Refused)
}
