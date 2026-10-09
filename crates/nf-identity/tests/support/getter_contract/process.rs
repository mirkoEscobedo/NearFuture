use std::{
    ffi::OsStr,
    path::Path,
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
pub fn run(script: &Path, args: &[&OsStr]) -> i32 {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ]);
    command.arg(script).args(args).creation_flags(0x08000000);
    let mut child = OwnedChild(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return status.code().expect("closed PowerShell exit required");
        }
        assert!(
            Instant::now() < deadline,
            "owned getter contract command exceeded five seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
