//! Owns an ACL helper so every early return closes its process lifetime.
use std::process::{Child, ExitStatus};
pub(crate) struct OwnedChild {
    child: Child,
    reaped: bool,
}
impl OwnedChild {
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child,
            reaped: false,
        }
    }
    /// Preserve the original kill-then-wait once while returning its actual outcomes.
    pub(crate) fn cleanup(&mut self) -> Option<(std::io::Result<()>, std::io::Result<ExitStatus>)> {
        if self.reaped {
            return None;
        }
        let killed = self.child.kill();
        let waited = self.child.wait();
        // This flag suppresses a second cleanup attempt, as in the original Drop.
        self.reaped = true;
        Some((killed, waited))
    }
    pub(crate) fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped = true;
        }
        Ok(status)
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    #[test]
    fn early_wait_fault_kills_and_reaps_owned_helper() {
        let child = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        let faulted: std::io::Result<()> = {
            let _owned = OwnedChild::new(child);
            Err(std::io::Error::other("injected owned wait fault"))
        };
        assert!(faulted.is_err());
        let probe = format!(
            "try {{ $null = [System.Diagnostics.Process]::GetProcessById({pid}); exit 1 }} catch {{ exit 0 }}"
        );
        let gone = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(probe)
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        // Cleanup the deliberately reproduced orphan even when RED fails.
        if !gone.success() {
            let cleanup = format!("Stop-Process -Id {pid} -Force -ErrorAction SilentlyContinue");
            let _ = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg(cleanup)
                .creation_flags(0x08000000)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        assert!(gone.success(), "Owned helper survived early wait fault");
    }
}
