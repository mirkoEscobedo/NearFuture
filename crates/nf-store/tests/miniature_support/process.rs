#![allow(dead_code)]
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
pub struct OwnedChild {
    child: Child,
    reaped: bool,
}
impl OwnedChild {
    pub fn spawn(command: &mut Command) -> Self {
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("owned miniature test child spawn");
        Self {
            child,
            reaped: false,
        }
    }
    pub fn wait(&mut self, duration: Duration) -> ExitStatus {
        let deadline = Instant::now() + duration;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                self.reaped = true;
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "owned child deadline; Drop kills/reaps"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn await_marker(&mut self, path: &Path, duration: Duration) {
        let deadline = Instant::now() + duration;
        loop {
            if path.exists() && read_bounded(path, 16).unwrap_or_default() == b"READY\n" {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "owned child exited before its actual boundary marker"
            );
            assert!(
                Instant::now() < deadline,
                "owned child boundary deadline; Drop kills/reaps"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn kill_and_reap(&mut self) {
        self.child.kill().unwrap();
        let status = self.child.wait().unwrap();
        self.reaped = true;
        assert!(!status.success());
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
pub fn read_bounded(path: &Path, max: usize) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = vec![0; max + 1];
    let mut count = 0;
    while count < bytes.len() {
        let n = file.read(&mut bytes[count..])?;
        if n == 0 {
            break;
        }
        count += n;
    }
    if count > max {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bounded miniature test input",
        ));
    }
    bytes.truncate(count);
    Ok(bytes)
}
pub fn block_at(path: &Path) {
    use std::io::Write;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"READY\n").unwrap();
    file.sync_all().unwrap();
    let deadline = Instant::now() + Duration::from_secs(45);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    std::process::exit(91)
}
