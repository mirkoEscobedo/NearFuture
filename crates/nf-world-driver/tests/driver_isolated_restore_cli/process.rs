//! Direct process-handle receipts for a source-audited, descendant-free CLI route.
//! No process-group flag, guessed PID census, or deleting failure destructor.
use super::{Fixture, capture::{bounded, hex, new_file, receipt}};
use nf_store::miniature::MiniatureKnownFrontiers;
use std::{fs, path::{Path, PathBuf}, process::{Child, Command, ExitStatus, Stdio}, time::{Duration, Instant}};

const STDOUT_LIMIT: u64 = 4096;
const STDERR_LIMIT: u64 = 1024;
const CLEANUP_LIMIT: Duration = Duration::from_secs(2);

pub struct Result {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

struct OwnedChild {
    child: Child,
    status: Option<ExitStatus>,
    deadline: Instant,
    cleanup_deadline: Option<Instant>,
    cleanup_receipt: PathBuf,
}

impl OwnedChild {
    fn cleanup(&mut self) -> String {
        let limit = *self.cleanup_deadline.get_or_insert_with(||
            (Instant::now() + CLEANUP_LIMIT).min(self.deadline));
        let kill = self.child.kill().map_err(|error| error.kind());
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.status = Some(status);
                    return format!("kill_result={kill:?};direct_try_wait=Some({status:?});owned_handle_reaped;descendant_zero=unproved_without_closed_route_audit");
                }
                Err(error) => return format!("INSUFFICIENT;kill_result={kill:?};try_wait_error={:?};no_zero_claim", error.kind()),
                Ok(None) => (),
            }
            let remaining = limit.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return format!("INSUFFICIENT;kill_result={kill:?};direct_try_wait=None;cleanup_clock_exhausted;no_zero_claim");
            }
            std::thread::sleep(remaining.min(Duration::from_millis(10)));
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.status.is_none() {
            let observation = self.cleanup();
            // Cleanup must never panic while unwinding. Any missing receipt or
            // unreaped handle remains INSUFFICIENT, never a passing zero proof.
            if let Ok(mut file) = fs::OpenOptions::new().write(true).create_new(true).open(&self.cleanup_receipt) {
                use std::io::Write;
                let _ = writeln!(file, "{observation}");
                let _ = file.sync_all();
            }
        }
    }
}

pub fn run(
    executable: &Path,
    fixture: &Fixture,
    label: &str,
    destination: &Path,
    known: MiniatureKnownFrontiers,
    confirmed: bool,
    deadline: Instant,
) -> Result {
    assert!(label.len() <= 32 && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'));
    let stdout = fixture.control.join(format!("{label}.stdout.raw"));
    let stderr = fixture.control.join(format!("{label}.stderr.raw"));
    let mut command = Command::new(executable);
    command.current_dir(&fixture.data)
        .stdin(Stdio::null()).stdout(Stdio::from(new_file(&stdout))).stderr(Stdio::from(new_file(&stderr)))
        .arg("restore-isolated").arg("--backup").arg(&fixture.backup)
        .arg("--destination").arg(destination).args([
            "--universe", &hex(known.storage.scope.universe.as_bytes()),
            "--history", &hex(known.storage.scope.history.as_bytes()),
            "--min-event", &known.storage.event_sequence.0.to_string(),
            "--min-store", &known.storage.store_revision.to_string(),
            "--min-membership", &known.storage.membership_revision.unwrap().to_string(),
            "--min-term", &known.minimum_authority_term.0.to_string(),
        ]);
    if confirmed {
        command.args(["--confirm", "NewReadOnlyCopy"]);
    }
    let observe_until = deadline.checked_sub(CLEANUP_LIMIT).unwrap();
    assert!(Instant::now() < observe_until, "reserve actual cleanup in original clock before spawn");
    let mut owned = OwnedChild {
        child: command.spawn().expect("one actual Cargo-built foreground CLI child"),
        status: None, deadline, cleanup_deadline: None,
        cleanup_receipt: fixture.control.join(format!("{label}.cleanup.public")),
    };
    drop(command); // Parent redirection handles close; no pipes or reader threads.
    let status = loop {
        let over = fs::metadata(&stdout).unwrap().len() > STDOUT_LIMIT
            || fs::metadata(&stderr).unwrap().len() > STDERR_LIMIT;
        if over || Instant::now() >= observe_until {
            let observation = owned.cleanup();
            receipt(&fixture.control.join(format!("{label}.failure.public")), &format!("output_cap={over};{observation};all originals retained"));
            panic!("bounded CLI execution refused; retained evidence, no passing cleanup inference");
        }
        match owned.child.try_wait() {
            Ok(Some(status)) => {
                owned.status = Some(status);
                break status;
            }
            Ok(None) => (),
            Err(error) => panic!("actual CLI wait failed: {error}; evidence retained"),
        }
        let remaining = observe_until.saturating_duration_since(Instant::now());
        std::thread::sleep(remaining.min(Duration::from_millis(10)));
    };
    // This records the observed ExitStatus returned by actual Child::try_wait,
    // not a boolean set after requesting termination or a containment flag.
    receipt(&fixture.control.join(format!("{label}.reap.public")), &format!("direct_try_wait=Some({status:?});owned_handle_reaped;descendant_zero=unproved_without_closed_route_audit"));
    drop(owned);
    let output = bounded(&stdout, STDOUT_LIMIT);
    let errors = bounded(&stderr, STDERR_LIMIT);
    super::check(deadline);
    Result { status, stdout: output, stderr: errors }
}
