use std::{
    ffi::OsString,
    io::Read,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
const PIPE_BYTES: usize = 65_536;
const LINE_BYTES: usize = 4096;
const LINE_COUNT: usize = 64;
#[derive(Clone, Debug)]
struct Line {
    text: String,
    observed: Instant,
}
struct Pipe {
    retained: Arc<Mutex<Vec<u8>>>,
    worker: Option<JoinHandle<()>>,
}
pub struct OwnedPeer {
    child: Child,
    output: Receiver<Line>,
    seen: Vec<Line>,
    stdout: Pipe,
    stderr: Pipe,
    overflow: Arc<AtomicBool>,
}
impl OwnedPeer {
    pub fn spawn(arguments: &[OsString]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nf-portal"));
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // Hidden console; inherits the supervising Job Object.
        }
        let mut child = command.spawn().expect("owned production peer process");
        let overflow = Arc::new(AtomicBool::new(false));
        let (send, output) = sync_channel(LINE_COUNT);
        let stdout_source = child.stdout.take().unwrap();
        let stderr_source = child.stderr.take().unwrap();
        let empty_pipe = || Pipe {
            retained: Arc::new(Mutex::new(Vec::new())),
            worker: None,
        };
        let mut owned = Self {
            child,
            output,
            seen: Vec::with_capacity(LINE_COUNT),
            stdout: empty_pipe(),
            stderr: empty_pipe(),
            overflow,
        };
        // The exact child already has a Drop owner if either reader thread creation fails.
        owned.stdout = drain(stdout_source, Some(send), Arc::clone(&owned.overflow));
        owned.stderr = drain(stderr_source, None, Arc::clone(&owned.overflow));
        owned
    }
    pub fn id(&self) -> u32 {
        self.child.id()
    }
    pub fn line_before(&mut self, prefix: &str, until: Instant) -> Option<(String, Instant)> {
        if let Some(line) = self.seen.iter().find(|line| line.text.starts_with(prefix)) {
            return Some((line.text.clone(), line.observed));
        }
        loop {
            let remaining = until.checked_duration_since(Instant::now())?;
            let line = self.output.recv_timeout(remaining).ok()?;
            if self.seen.len() == LINE_COUNT {
                self.overflow.store(true, Ordering::Relaxed);
                return None;
            }
            let matches = line.text.starts_with(prefix);
            let answer = matches.then(|| (line.text.clone(), line.observed));
            self.seen.push(line);
            if matches {
                return answer;
            }
        }
    }
    pub fn healthy_running(&mut self) {
        assert!(
            !self.overflow.load(Ordering::Relaxed),
            "bounded peer output overflow; observed={:?}; inspected={:?}; {}",
            self.seen,
            Instant::now(),
            self.logs()
        );
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "peer ended during setup/oracle; observed={:?}; inspected={:?}; {}",
            self.seen,
            Instant::now(),
            self.logs()
        );
        assert!(
            !self.logs().contains("NF_PORTAL_ERROR"),
            "peer setup/protocol failure; observed={:?}; inspected={:?}; {}",
            self.seen,
            Instant::now(),
            self.logs()
        );
    }
    pub fn finish_before(&mut self, until: Instant) -> ExitStatus {
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                self.join_readers();
                assert!(
                    !self.overflow.load(Ordering::Relaxed),
                    "bounded peer output overflow"
                );
                return status;
            }
            assert!(
                Instant::now() < until,
                "owned process completion watchdog: {}",
                self.logs()
            );
            // Completion polling only; network delivery is gated exclusively by actual output events.
            thread::park_timeout(Duration::from_millis(25));
        }
    }
    pub fn count_output(&self, prefix: &str) -> usize {
        self.stdout_text()
            .lines()
            .filter(|line| line.starts_with(prefix))
            .count()
    }
    /// Failure-only context from existing public stdout and actual reader observations.
    /// It neither consumes queued lines nor exposes stderr payloads or hidden owner state.
    pub fn public_context(&self) -> String {
        const PHASES: [&str; 8] = [
            "NF_PORTAL_PREPARED",
            "NF_PORTAL_READY",
            "NF_PORTAL_RECEIPT_AUTHENTICATED",
            "NF_PORTAL_SUBSCRIBED",
            "NF_PORTAL_SUBSCRIPTION_RESPONSE_SENT",
            "NF_PORTAL_STATUS",
            "NF_PORTAL_UNSUPPORTED",
            "NF_PORTAL_ENDED",
        ];
        let seen: Vec<_> = self
            .seen
            .iter()
            .filter(|line| PHASES.iter().any(|phase| line.text.starts_with(phase)))
            .map(|line| (line.text.as_str(), line.observed))
            .collect();
        let output = self.stdout_text();
        let retained: Vec<_> = output
            .lines()
            .filter(|line| PHASES.iter().any(|phase| line.starts_with(phase)))
            .take(LINE_COUNT)
            .collect();
        let counts: [usize; 8] = std::array::from_fn(|index| {
            output
                .lines()
                .filter(|line| line.starts_with(PHASES[index]))
                .count()
        });
        let stderr = self
            .stderr
            .retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let errors = String::from_utf8_lossy(&stderr)
            .lines()
            .filter(|line| line.starts_with("NF_PORTAL_ERROR "))
            .count();
        format!(
            "seen_public={seen:?}; retained_public={retained:?}; phases={PHASES:?}; counts={counts:?}; error_frames={errors}; captured_bytes=({},{}); overflow={}",
            output.len(),
            stderr.len(),
            self.overflow.load(Ordering::Relaxed)
        )
    }
    pub fn logs(&self) -> String {
        let stderr = self
            .stderr
            .retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        format!(
            "stdout={} stderr={}",
            self.stdout_text(),
            String::from_utf8_lossy(&stderr)
        )
    }
    fn stdout_text(&self) -> String {
        let bytes = self
            .stdout
            .retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        String::from_utf8_lossy(&bytes).into_owned()
    }
    fn join_readers(&mut self) {
        for pipe in [&mut self.stdout, &mut self.stderr] {
            if let Some(worker) = pipe.worker.take() {
                worker.join().expect("owned pipe reader joined");
            }
        }
    }
}
impl Drop for OwnedPeer {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        for pipe in [&mut self.stdout, &mut self.stderr] {
            if let Some(worker) = pipe.worker.take() {
                let _ = worker.join();
            }
        }
    }
}
fn drain(
    mut reader: impl Read + Send + 'static,
    send: Option<SyncSender<Line>>,
    overflow: Arc<AtomicBool>,
) -> Pipe {
    let retained = Arc::new(Mutex::new(Vec::with_capacity(PIPE_BYTES)));
    let capture = Arc::clone(&retained);
    let worker = thread::spawn(move || {
        let mut buffer = [0; 1024];
        let mut line = Vec::with_capacity(LINE_BYTES);
        let mut line_overflow = false;
        let mut total = 0usize;
        loop {
            let length = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => length,
                Err(_) => {
                    overflow.store(true, Ordering::Relaxed);
                    break;
                }
            };
            total = total.saturating_add(length);
            if total > PIPE_BYTES {
                overflow.store(true, Ordering::Relaxed);
            }
            let mut kept = capture
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let keep = length.min(PIPE_BYTES.saturating_sub(kept.len()));
            kept.extend_from_slice(&buffer[..keep]);
            drop(kept);
            if let Some(send) = &send {
                for byte in &buffer[..length] {
                    if *byte == b'\n' {
                        if !line_overflow {
                            match String::from_utf8(std::mem::take(&mut line)) {
                                Ok(text) => {
                                    if send
                                        .try_send(Line {
                                            text: text.trim_end_matches('\r').to_owned(),
                                            observed: Instant::now(),
                                        })
                                        .is_err()
                                    {
                                        overflow.store(true, Ordering::Relaxed);
                                    }
                                }
                                Err(_) => {
                                    overflow.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                        line.clear();
                        line_overflow = false;
                    } else if line.len() < LINE_BYTES && !line_overflow {
                        line.push(*byte);
                    } else {
                        line.clear();
                        line_overflow = true;
                        overflow.store(true, Ordering::Relaxed);
                    }
                }
            }
        }
        if !line.is_empty() || line_overflow {
            overflow.store(true, Ordering::Relaxed);
        }
    });
    Pipe {
        retained,
        worker: Some(worker),
    }
}
