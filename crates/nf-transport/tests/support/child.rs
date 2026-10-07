#![allow(dead_code)] // The same owned child guard is compiled by separate focused process tests.
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};
pub struct OwnedChild {
    child: Child,
    out: PathBuf,
    err: PathBuf,
    reaped: bool,
}
impl OwnedChild {
    pub fn spawn(args: &[String], root: &Path, name: &str) -> Self {
        let out = root.join(format!("{name}.out"));
        let err = root.join(format!("{name}.err"));
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out)
            .unwrap();
        let error = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&err)
            .unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_nf-peer"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(output)
            .stderr(error)
            .spawn()
            .unwrap();
        Self {
            child,
            out,
            err,
            reaped: false,
        }
    }
    pub fn output(&self) -> String {
        read(&self.out)
    }
    pub fn error(&self) -> String {
        read(&self.err)
    }
    pub fn wait(&mut self, limit: Duration) -> ExitStatus {
        let deadline = Instant::now() + limit;
        loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                self.reaped = true;
                return s;
            }
            assert!(
                Instant::now() < deadline,
                "exact child exit deadline; owned handle will be killed and reaped"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn ready(&mut self, limit: Duration) -> (String, String) {
        let deadline = Instant::now() + limit;
        loop {
            for line in self.output().lines() {
                if let Some(rest) = line.strip_prefix("NF_PEER_READY ") {
                    let fields: Vec<_> = rest.split(' ').collect();
                    assert_eq!(fields.len(), 2);
                    return (fields[0].to_owned(), fields[1].to_owned());
                }
            }
            if let Some(s) = self.child.try_wait().unwrap() {
                self.reaped = true;
                panic!("child exited before actual readiness: {s} {}", self.error());
            }
            assert!(
                Instant::now() < deadline,
                "actual output readiness deadline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn terminate(&mut self) {
        if !self.reaped {
            match self.child.try_wait().unwrap() {
                Some(_) => {}
                None => {
                    self.child.kill().unwrap();
                    self.child.wait().unwrap();
                }
            }
            self.reaped = true;
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.terminate();
    }
}
fn read(path: &Path) -> String {
    let mut f = File::open(path).unwrap();
    let mut bytes = [0; 4097];
    let mut n = 0;
    while n < bytes.len() {
        let got = f.read(&mut bytes[n..]).unwrap();
        if got == 0 {
            break;
        }
        n += got;
    }
    assert!(n <= 4096, "owned CLI output cap");
    String::from_utf8(bytes[..n].to_vec()).unwrap()
}
