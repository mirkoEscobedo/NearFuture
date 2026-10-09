use super::{ChildState, Process};
use crate::model::IdentityError;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Copy)]
pub(super) enum Mode {
    WrongAck,
    EarlyExit,
    Nonzero,
    Stderr,
    Trailing,
    Oversize,
    FullPipes,
    Silent,
    Descendant,
}
impl Mode {
    fn literal(self) -> &'static str {
        match self {
            Self::WrongAck => "WrongAck",
            Self::EarlyExit => "EarlyExit",
            Self::Nonzero => "Nonzero",
            Self::Stderr => "Stderr",
            Self::Trailing => "Trailing",
            Self::Oversize => "Oversize",
            Self::FullPipes => "FullPipes",
            Self::Silent => "Silent",
            Self::Descendant => "Descendant",
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Stage {
    Monitor,
    Pipes,
    Writer,
    Stdout,
    Stderr,
}
pub(super) type Held = Arc<Mutex<ChildState>>;
pub(super) struct Setup {
    pub stage: Stage,
    pub observed: Arc<Mutex<Option<Held>>>,
}
impl Setup {
    pub fn capture(&self, child: &Held) -> Result<(), IdentityError> {
        *self
            .observed
            .lock()
            .map_err(|_| IdentityError::PrivateStorage)? = Some(Arc::clone(child));
        Ok(())
    }
    pub fn check(&self, stage: Stage) -> Result<(), IdentityError> {
        if self.stage == stage {
            Err(IdentityError::PrivateStorage)
        } else {
            Ok(())
        }
    }
}
pub(super) fn fixture(mode: Mode, setup: Option<Setup>) -> Result<Process, IdentityError> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(5))
        .ok_or(IdentityError::PrivateStorage)?;
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
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/acl_session/engine-fixture.ps1"
        ))
        .args(["-Mode", mode.literal()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Process::start_owned(command, deadline, setup)
}
pub(super) fn held(p: &Process) -> Held {
    let held = Arc::clone(&p.child);
    assert!(
        held.lock()
            .unwrap()
            .contained
            .child
            .try_wait()
            .unwrap()
            .is_none()
    );
    held
}
pub(super) fn reaped(held: Held) {
    let mut s = held.lock().unwrap();
    assert!(s.contained.child.try_wait().unwrap().is_some());
    assert_eq!(s.contained.job.process_count().unwrap(), 0);
}
pub(super) struct Scratch(PathBuf);
impl Scratch {
    pub fn new() -> Self {
        let n: String = crate::keys::random_id()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        std::fs::create_dir_all(&base).unwrap();
        let p = base.join(format!("acl-engine-{n}"));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub(super) struct Identity {
    pub pid: u32,
    pub ticks: u64,
}
pub(super) fn parse_ready(bytes: &[u8]) -> [Identity; 2] {
    assert!(bytes.len() <= 64);
    let text = std::str::from_utf8(bytes).unwrap();
    assert!(text.ends_with('\n'));
    let v: Vec<_> = text.trim_end_matches('\n').split(' ').collect();
    assert_eq!(v.len(), 5);
    assert_eq!(v[0], "READY");
    let ids = [
        Identity {
            pid: v[1].parse().unwrap(),
            ticks: v[2].parse().unwrap(),
        },
        Identity {
            pid: v[3].parse().unwrap(),
            ticks: v[4].parse().unwrap(),
        },
    ];
    assert!(
        ids[0].pid > 0
            && ids[1].pid > 0
            && ids[0].pid != ids[1].pid
            && ids[0].ticks > 0
            && ids[1].ticks > 0
    );
    ids
}
pub(super) fn ready(p: &Process) -> [Identity; 2] {
    let ids = parse_ready(&p.line().expect("actual descendant readiness"));
    assert_eq!(p.child.lock().unwrap().contained.child.id(), ids[0].pid);
    ids
}
use super::fixture_runner as observer_runner;
pub(super) fn observe(id: &Identity, exited: bool) {
    use std::ffi::OsStr;
    let script = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/acl_session/observe-identity.ps1"
    ));
    let pid = id.pid.to_string();
    let ticks = id.ticks.to_string();
    let mut args = vec![
        OsStr::new("-KnownProcess"),
        OsStr::new(&pid),
        OsStr::new("-CreationTicks"),
        OsStr::new(&ticks),
    ];
    if exited {
        args.push(OsStr::new("-ExpectExited"));
    }
    assert_eq!(
        observer_runner::run(script, &args),
        0,
        "actual known PID creation identity observation"
    );
}
