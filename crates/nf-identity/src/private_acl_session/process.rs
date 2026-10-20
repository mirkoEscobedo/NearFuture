use super::protocol::OUTPUT_LIMIT;
use crate::model::IdentityError;
use std::{
    io::{Read, Write},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use subc_jobobject::ContainedChild;

type Worker = JoinHandle<Result<(), IdentityError>>;
struct ChildState {
    contained: ContainedChild<Child>,
    status: Option<ExitStatus>,
}
pub(super) struct Process {
    child: Arc<Mutex<ChildState>>,
    failed: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    monitor: Option<JoinHandle<()>>,
    workers: Vec<Worker>,
    input: Option<SyncSender<Option<Vec<u8>>>>,
    output: Option<Receiver<Vec<u8>>>,
    deadline: Instant,
    closed: bool,
}
impl Process {
    pub fn start() -> Result<Self, IdentityError> {
        let started = Instant::now();
        let deadline = started
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
                "/src/private-acl-session.ps1"
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(not(test))]
        {
            Self::start_owned(command, deadline)
        }
        #[cfg(test)]
        {
            Self::start_owned(command, deadline, None)
        }
    }
    fn start_owned(
        mut command: Command,
        deadline: Instant,
        #[cfg(test)] setup: Option<fixture_support::Setup>,
    ) -> Result<Self, IdentityError> {
        let contained = subc_jobobject::spawn_contained(&mut command)
            .map_err(|_| IdentityError::PrivateStorage)?;
        // Child and kill-on-close Job are owned before extraction or any fallible thread setup.
        let mut owner = Self {
            child: Arc::new(Mutex::new(ChildState {
                contained,
                status: None,
            })),
            failed: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
            monitor: None,
            workers: Vec::new(),
            input: None,
            output: None,
            deadline,
            closed: false,
        };
        #[cfg(test)]
        if let Some(setup) = &setup {
            setup.capture(&owner.child)?;
            setup.check(fixture_support::Stage::Monitor)?;
        }
        let child = Arc::clone(&owner.child);
        let failed = Arc::clone(&owner.failed);
        let stopped = Arc::clone(&owner.stopped);
        owner.monitor = Some(
            thread::Builder::new()
                .spawn(move || {
                    while !stopped.load(Ordering::Acquire) {
                        let mut state = child
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        match state.contained.child.try_wait() {
                            Ok(Some(status)) => {
                                if !status.success() {
                                    failed.store(true, Ordering::Release);
                                }
                                state.status = Some(status);
                                break;
                            }
                            Err(_) => {
                                failed.store(true, Ordering::Release);
                            }
                            Ok(None) => {}
                        }
                        if Instant::now() >= deadline || failed.load(Ordering::Acquire) {
                            failed.store(true, Ordering::Release);
                            let _ = state.contained.job.terminate();
                            let _ = state.contained.child.kill();
                            state.status = state.contained.child.wait().ok();
                            break;
                        }
                        drop(state);
                        thread::sleep(Duration::from_millis(5));
                    }
                })
                .map_err(|_| IdentityError::PrivateStorage)?,
        );
        #[cfg(test)]
        if let Some(setup) = &setup {
            setup.check(fixture_support::Stage::Pipes)?;
        }
        let (stdin, stdout, stderr) = {
            let mut state = owner
                .child
                .lock()
                .map_err(|_| IdentityError::PrivateStorage)?;
            (
                state.contained.child.stdin.take(),
                state.contained.child.stdout.take(),
                state.contained.child.stderr.take(),
            )
        };
        let mut stdin = stdin.ok_or(IdentityError::PrivateStorage)?;
        let mut stdout = stdout.ok_or(IdentityError::PrivateStorage)?;
        let mut stderr = stderr.ok_or(IdentityError::PrivateStorage)?;
        #[cfg(test)]
        if let Some(setup) = &setup {
            setup.check(fixture_support::Stage::Writer)?;
        }
        let (input, requests) = mpsc::sync_channel::<Option<Vec<u8>>>(1);
        owner.input = Some(input);
        let failed = Arc::clone(&owner.failed);
        owner.workers.push(
            thread::Builder::new()
                .spawn(move || {
                    while let Ok(Some(frame)) = requests.recv() {
                        if stdin
                            .write_all(&frame)
                            .and_then(|()| stdin.flush())
                            .is_err()
                        {
                            failed.store(true, Ordering::Release);
                            return Err(IdentityError::PrivateStorage);
                        }
                    }
                    drop(stdin);
                    Ok(())
                })
                .map_err(|_| IdentityError::PrivateStorage)?,
        );
        #[cfg(test)]
        if let Some(setup) = &setup {
            setup.check(fixture_support::Stage::Stdout)?;
        }
        let (lines, output) = mpsc::sync_channel(7);
        owner.output = Some(output);
        let failed = Arc::clone(&owner.failed);
        owner.workers.push(
            thread::Builder::new()
                .spawn(move || {
                    let result = (|| {
                        let mut total = 0;
                        let mut line = Vec::with_capacity(OUTPUT_LIMIT);
                        loop {
                            let mut byte = [0];
                            if stdout
                                .read(&mut byte)
                                .map_err(|_| IdentityError::PrivateStorage)?
                                == 0
                            {
                                return if line.is_empty() {
                                    Ok(())
                                } else {
                                    Err(IdentityError::PrivateStorage)
                                };
                            }
                            total += 1;
                            if total > OUTPUT_LIMIT {
                                return Err(IdentityError::PrivateStorage);
                            }
                            line.push(byte[0]);
                            if byte[0] == b'\n' {
                                lines
                                    .try_send(std::mem::take(&mut line))
                                    .map_err(|_| IdentityError::PrivateStorage)?;
                            }
                        }
                    })();
                    if result.is_err() {
                        failed.store(true, Ordering::Release);
                    }
                    result
                })
                .map_err(|_| IdentityError::PrivateStorage)?,
        );
        #[cfg(test)]
        if let Some(setup) = &setup {
            setup.check(fixture_support::Stage::Stderr)?;
        }
        let failed = Arc::clone(&owner.failed);
        owner.workers.push(
            thread::Builder::new()
                .spawn(move || {
                    let result = match stderr.read(&mut [0]) {
                        Ok(0) => Ok(()),
                        _ => Err(IdentityError::PrivateStorage),
                    };
                    if result.is_err() {
                        failed.store(true, Ordering::Release);
                    }
                    result
                })
                .map_err(|_| IdentityError::PrivateStorage)?,
        );
        owner.live()?;
        Ok(owner)
    }
    fn live(&self) -> Result<(), IdentityError> {
        if self.failed.load(Ordering::Acquire) || Instant::now() >= self.deadline {
            return Err(IdentityError::PrivateStorage);
        }
        Ok(())
    }
    fn send(&self, mut frame: Option<Vec<u8>>) -> Result<(), IdentityError> {
        loop {
            self.live()?;
            match self
                .input
                .as_ref()
                .ok_or(IdentityError::PrivateStorage)?
                .try_send(frame)
            {
                Ok(()) => return Ok(()),
                Err(TrySendError::Full(returned)) => {
                    frame = returned;
                    thread::sleep(Duration::from_millis(5));
                }
                Err(TrySendError::Disconnected(_)) => return Err(IdentityError::PrivateStorage),
            }
        }
    }
    fn line(&self) -> Result<Vec<u8>, IdentityError> {
        self.live()?;
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(IdentityError::PrivateStorage)?;
        let result = self
            .output
            .as_ref()
            .ok_or(IdentityError::PrivateStorage)?
            .recv_timeout(remaining)
            .map_err(|_| IdentityError::PrivateStorage)?;
        self.live()?;
        Ok(result)
    }
    pub fn request(&self, frame: Vec<u8>, sequence: u8) -> Result<(), IdentityError> {
        self.send(Some(frame))?;
        if self.line()? != format!("ACK {sequence}\n").as_bytes() {
            return Err(IdentityError::PrivateStorage);
        }
        Ok(())
    }
    pub fn finish(
        mut self,
        mode: super::protocol::Completion,
        sequence: u8,
    ) -> Result<(), IdentityError> {
        self.send(Some(super::protocol::completion(mode, sequence)?))?;
        self.send(None)?;
        self.input.take();
        if self.line()? != b"DONE\n" {
            return Err(IdentityError::PrivateStorage);
        }
        loop {
            self.live()?;
            let status = self
                .child
                .lock()
                .map_err(|_| IdentityError::PrivateStorage)?
                .status;
            if let Some(status) = status {
                if !status.success() {
                    return Err(IdentityError::PrivateStorage);
                }
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        // Reap/terminate before joins, including inherited pipes held by any descendant.
        self.cleanup()?;
        if self
            .output
            .as_ref()
            .ok_or(IdentityError::PrivateStorage)?
            .try_recv()
            .is_ok()
        {
            return Err(IdentityError::PrivateStorage);
        }
        self.live()?;
        Ok(())
    }
    fn cleanup(&mut self) -> Result<(), IdentityError> {
        if self.closed {
            return Ok(());
        }
        self.stopped.store(true, Ordering::Release);
        self.input.take();
        let mut failed = false;
        {
            let mut state = self
                .child
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            failed |= state.contained.job.terminate().is_err();
            let _ = state.contained.child.kill();
            failed |= state.contained.child.wait().is_err();
            loop {
                match state.contained.job.process_count() {
                    Ok(0) => break,
                    Ok(_) if Instant::now() < self.deadline => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    _ => {
                        failed = true;
                        break;
                    }
                }
            }
        }
        if let Some(monitor) = self.monitor.take() {
            failed |= monitor.join().is_err();
        }
        for worker in self.workers.drain(..) {
            failed |= !matches!(worker.join(), Ok(Ok(())));
        }
        self.closed = true;
        if failed {
            Err(IdentityError::PrivateStorage)
        } else {
            Ok(())
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
#[path = "process_contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
#[path = "fixture_lifecycle_tests.rs"]
mod fixture_lifecycle_tests;
#[cfg(test)]
#[path = "fixture_output_tests.rs"]
mod fixture_output_tests;
#[cfg(test)]
#[path = "../../tests/support/getter_contract/process.rs"]
mod fixture_runner;

#[cfg(test)]
#[path = "fixture_setup_tests.rs"]
mod fixture_setup_tests;
#[cfg(test)]
#[path = "fixture_support.rs"]
mod fixture_support;
