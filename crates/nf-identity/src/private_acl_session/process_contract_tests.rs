use super::fixture_runner as original_runner;
use super::{ChildState, Process};
use crate::{
    model::IdentityError,
    private_acl_session::protocol::{self, Completion},
};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Fixture {
    _scratch: Scratch,
    root: PathBuf,
    existing: PathBuf,
    file: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let name: String = crate::keys::random_id()
            .unwrap()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
        fs::create_dir_all(&base).unwrap();
        let scratch = base.join(format!("acl-process-{name}"));
        fs::create_dir(&scratch).unwrap();
        let scratch = Scratch(scratch);
        let root = scratch.0.join("private");
        fs::create_dir(&root).unwrap();
        let existing = root.join("existing");
        fs::write(&existing, b"public fixture").unwrap();
        let file = root.join("candidate");
        let script = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/getter_contract/original-private-acl.ps1"
        ));
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(script).unwrap())),
            "d83b47582ed20336ac79ff674adfb40846bed3ccec4416aad7ef9107bf3ad0aa"
        );
        for path in [&root, &existing] {
            assert_eq!(
                original_runner::run(
                    script,
                    &[
                        OsStr::new("-PrivatePath"),
                        path.as_os_str(),
                        OsStr::new("-Initialize")
                    ]
                ),
                0,
                "original initializer setup failed"
            );
        }
        Self {
            _scratch: scratch,
            root,
            existing,
            file,
        }
    }
    fn prefix(&self, process: &Process, count: u8) {
        for sequence in 1..=count {
            let newly_opened = if sequence == 4 {
                Some(
                    fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&self.file)
                        .expect("genuinely new candidate before I4 initialization"),
                )
            } else {
                None
            };
            let paths = match sequence {
                1 => vec![self.root.as_path(), self.existing.as_path()],
                4 => vec![self.file.as_path()],
                5 => vec![
                    self.root.as_path(),
                    self.existing.as_path(),
                    self.file.as_path(),
                ],
                _ => vec![self.root.as_path()],
            };
            let operation = if sequence == 4 { "I" } else { "V" };
            let outcome = process.request(
                protocol::frame(sequence, sequence == 4, &paths).unwrap(),
                sequence,
            );
            assert_eq!(
                outcome,
                Ok(()),
                "fixed helper prefix count={count} sequence={sequence} operation={operation}"
            );
            if let Some(mut candidate) = newly_opened {
                std::io::Write::write_all(&mut candidate, b"public fixture")
                    .expect("public payload only after real I4 ACK");
                candidate
                    .sync_all()
                    .expect("public payload sync only after real I4 ACK");
            }
        }
    }
}
fn held_live(process: &Process) -> Arc<Mutex<ChildState>> {
    let held = Arc::clone(&process.child);
    let mut state = held.lock().unwrap();
    assert!(state.contained.child.id() > 0);
    assert!(state.contained.child.try_wait().unwrap().is_none());
    drop(state);
    held
}
fn reaped_before_job_closes(held: Arc<Mutex<ChildState>>) {
    let mut state = held.lock().unwrap();
    assert!(
        state.contained.child.try_wait().unwrap().is_some(),
        "actual helper must be reaped while its Job handle remains held"
    );
    assert_eq!(state.contained.job.process_count().unwrap(), 0);
}

#[test]
fn fixed_helper_accepts_readonly_completion_and_reaps_before_job_close() {
    let fixture = Fixture::new();
    let process = Process::start().expect("production fixed helper setup");
    let held = held_live(&process);
    fixture.prefix(&process, 2);
    process
        .finish(Completion::ReadOnly, 2)
        .expect("R2, EOF, DONE and successful exit");
    reaped_before_job_closes(held);
}

#[test]
fn fixed_helper_refuses_eof_without_terminal_after_every_valid_prefix() {
    for count in 0..=6 {
        let fixture = Fixture::new();
        let mut process = Process::start().expect("production fixed helper setup");
        let held = held_live(&process);
        fixture.prefix(&process, count);
        process.input.take();
        assert_eq!(process.line(), Err(IdentityError::PrivateStorage));
        drop(process);
        reaped_before_job_closes(held);
    }
}

#[test]
fn fixed_helper_refuses_wrong_stage_count_duplicate_and_trailing_input() {
    let fixture = Fixture::new();
    let root = fixture.root.to_str().unwrap();
    let file = fixture.existing.to_str().unwrap();
    let cases = [
        (0, format!("V 2 1\n{root}\n"), 2),
        (0, format!("I 1 1\n{root}\n"), 1),
        (0, "V 1 0\n".to_owned(), 1),
        (0, "V 1 66\n".to_owned(), 1),
        (0, format!("V 1 2\n{root}\n{root}\n"), 1),
        (1, format!("V 2 2\n{root}\n{file}\n"), 2),
        (3, format!("I 4 2\n{file}\n{root}\n"), 4),
    ];
    for (prefix, frame, sequence) in cases {
        let process = Process::start().expect("production fixed helper setup");
        let held = held_live(&process);
        fixture.prefix(&process, prefix);
        assert_eq!(
            process.request(frame.into_bytes(), sequence),
            Err(IdentityError::PrivateStorage)
        );
        drop(process);
        reaped_before_job_closes(held);
    }
    let mut process = Process::start().expect("production fixed helper setup");
    let held = held_live(&process);
    fixture.prefix(&process, 2);
    process.send(Some(b"R 2\ntrailing\n".to_vec())).unwrap();
    process.send(None).unwrap();
    process.input.take();
    assert_eq!(process.line(), Err(IdentityError::PrivateStorage));
    drop(process);
    reaped_before_job_closes(held);
}
