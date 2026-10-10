#[path = "../chat_java_enqueue_support/mod.rs"]
mod fixture;
pub use fixture::{Fixture, hash, message_bytes};
use nf_ipc::{AuthenticatedSession, DiscoveryRecord, EndpointRole, FramePump};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct Owner {
    child: Option<Child>,
    pub pump: FramePump,
    pub session: AuthenticatedSession,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}
impl Owner {
    pub fn start(f: &Fixture, name: &str) -> Self {
        let config = f.config(1);
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let evidence = PathBuf::from(
            std::env::var_os("NF_CHAT_COMMAND_QUERY_EVIDENCE_DIR")
                .expect("explicit owned command-query evidence directory required"),
        );
        assert!(
            evidence.is_absolute(),
            "absolute command-query evidence directory required"
        );
        let evidence = fs::canonicalize(evidence).unwrap();
        assert!(fs::metadata(&evidence).unwrap().is_dir());
        let fixture_directory = fs::canonicalize(f.store_path.parent().unwrap()).unwrap();
        assert!(
            !evidence.starts_with(&fixture_directory),
            "capture directory must survive fixture drop"
        );
        let stdout_path = evidence.join("command-query.stdout");
        let stderr_path = evidence.join("command-query.stderr");
        let stdout = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stdout_path)
            .unwrap();
        let stderr = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stderr_path)
            .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command
            .stdin(Stdio::null())
            .arg("serve-chat-command")
            .arg(&f.vault_path)
            .arg(&f.saves_root)
            .arg(name)
            .arg(&f.store_path)
            .arg(&f.outbox_path)
            .args([
                hex(&config.universe),
                hex(&config.history),
                hex(&config.ruleset),
                hex(&config.content_policy),
                hex(&f.alice.public.peer),
                "0".into(),
                "1".into(),
                "0".into(),
                hex(f.bob.public.account.as_bytes()),
                hex(f.bob.public.device.as_bytes()),
                "6000".into(),
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        let mut child = UnconnectedChild(Some(command.spawn().unwrap()));
        let until = Instant::now() + Duration::from_secs(4);
        let record = loop {
            capture_sizes(&stdout_path, &stderr_path);
            assert!(
                child.0.as_mut().unwrap().try_wait().unwrap().is_none(),
                "CLI stopped before authenticated command/query setup"
            );
            if let Ok(record) = DiscoveryRecord::read(&f.vault, name) {
                break record;
            }
            assert!(
                Instant::now() < until,
                "bounded command-owner publication missing"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(record.principal().account, f.alice.public.account);
        assert_eq!(record.principal().device, f.alice.public.device);
        assert_eq!(record.config().universe, config.universe);
        assert_eq!(record.config().history, config.history);
        assert_ne!(record.config().runtime_session, 0);
        let socket = TcpStream::connect_timeout(&record.address(), Duration::from_secs(1)).unwrap();
        assert!(socket.peer_addr().unwrap().ip().is_loopback());
        assert_eq!(socket.peer_addr().unwrap(), record.address());
        let mut pump = FramePump::new(socket, 4096).unwrap();
        let hello = record.client(EndpointRole::Control).unwrap();
        pump.send(hello.hello()).unwrap();
        let proof = hello.respond(&receive(&mut pump)).unwrap();
        pump.send(proof.proof()).unwrap();
        let session = proof.finish(&receive(&mut pump)).unwrap();
        assert_eq!(session.runtime_session(), record.config().runtime_session);
        pump.activate(&session).unwrap();
        Self {
            child: child.0.take(),
            pump,
            session,
            stdout_path,
            stderr_path,
        }
    }
    pub fn exchange(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.pump.send(bytes).unwrap();
        receive(&mut self.pump)
    }
    pub fn finish(mut self) {
        let until = Instant::now() + Duration::from_secs(7);
        loop {
            capture_sizes(&self.stdout_path, &self.stderr_path);
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(
                Instant::now() < until,
                "bounded command-owner shutdown missing"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        // Root try_wait has reaped it. Regular-file captures do not wait for inherited pipe EOF.
        self.child.take();
        let stdout = read_capture(&self.stdout_path, until);
        let stderr = read_capture(&self.stderr_path, until);
        assert_eq!(
            String::from_utf8(stdout)
                .unwrap()
                .replace("\r\n", "\n")
                .trim(),
            "READY\nSTOPPED"
        );
        assert!(stderr.is_empty());
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = kill_and_reap(child);
        }
    }
}
struct UnconnectedChild(Option<Child>);
impl Drop for UnconnectedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = kill_and_reap(child);
        }
    }
}
fn receive(pump: &mut FramePump) -> Vec<u8> {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(bytes) = pump.poll(4096, 4096).unwrap() {
            return bytes;
        }
        assert!(
            Instant::now() < until,
            "bounded authenticated response missing"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Bounded unwind cleanup. The enclosing managed Job is the hard descendant backstop.
fn kill_and_reap(child: &mut Child) -> bool {
    let until = Instant::now() + Duration::from_secs(3);
    if matches!(child.try_wait(), Ok(Some(_))) {
        return true;
    }
    let _ = child.kill();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Err(_) => return false,
            Ok(None) => {}
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn capture_sizes(stdout: &Path, stderr: &Path) {
    for path in [stdout, stderr] {
        let metadata = fs::metadata(path).unwrap();
        assert!(
            metadata.is_file() && metadata.len() <= 4096,
            "CLI capture bound exceeded"
        );
    }
}
fn read_capture(path: &Path, until: Instant) -> Vec<u8> {
    assert!(Instant::now() < until, "CLI capture deadline exceeded");
    let metadata = fs::metadata(path).unwrap();
    assert!(
        metadata.is_file() && metadata.len() <= 4096,
        "CLI capture bound exceeded"
    );
    let mut bytes = Vec::with_capacity(4097);
    File::open(path)
        .unwrap()
        .take(4097)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 4096, "CLI capture bound exceeded");
    assert!(Instant::now() < until, "CLI capture deadline exceeded");
    bytes
}
