//! Linux-only, actual final standalone fsync refusal; retained public synthetic data.
#![cfg(target_os = "linux")]
mod miniature_restore_io_support;
mod miniature_support;
#[path = "miniature_restore_refusal_support/mod.rs"]
mod restore_fixture;

use miniature_restore_io_support::{
    bounded, clock, create, epoch_millis, process::OwnedTrace, trace, worker,
};
use nf_store::miniature::MiniatureReader;
use restore_fixture::{Entry, RetainedBackup, tree};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[test]
fn restore_final_sync_worker() {
    worker::run();
}

fn product_tree(root: &Path) -> BTreeMap<PathBuf, Entry> {
    tree(root)
        .into_iter()
        .filter(|(path, _)| !path.starts_with("control"))
        .collect()
}

fn traced(
    case: &RetainedBackup,
    mode: &str,
    deadline: Instant,
    epoch_deadline: u128,
    injection: Option<&trace::Cut>,
) -> miniature_restore_io_support::process::Run {
    clock(deadline - Duration::from_secs(2));
    let control = case.root.join("control").join(mode);
    fs::create_dir(&control).unwrap();
    let mut command = Command::new("strace");
    command
        .process_group(0)
        .current_dir(case.backup.parent().unwrap())
        .args([
            "--kill-on-exit",
            "-f",
            "-yy",
            "-s",
            "128",
            "-e",
            "trace=process,open,openat,openat2,close,write,fsync,fdatasync,setpgid,setsid,ptrace",
            "-o",
        ])
        .arg(control.join("trace.raw"));
    if let Some(cut) = injection {
        command.arg("-e").arg(format!(
            "inject={}:error=EIO:when={}",
            cut.syscall, cut.ordinal
        ));
    }
    command
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "restore_final_sync_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NF_RESTORE_IO_WORKER", mode)
        .env("NF_RESTORE_IO_DEADLINE_MS", epoch_deadline.to_string());
    let mut child = OwnedTrace::spawn(command, control, deadline);
    let (main, writer) = child.release();
    child.finish(main, writer)
}

#[test]
#[ignore = "requires explicit caller-owned Linux fault-qualification lane"]
fn isolated_restore_final_standalone_fsync_eio_retains_unqualified_output() {
    assert_eq!(
        std::env::var("NF_RESTORE_REQUIRE_FINAL_SYNC_FAULT").unwrap(),
        "1"
    );
    // The same one20sec clock includes fixture creation, both runs and cleanup.
    let started = Instant::now();
    let deadline = started + Duration::from_secs(20);
    let epoch_deadline = epoch_millis() + 18_000;
    let case = RetainedBackup::new("final-standalone-fsync");
    clock(deadline - Duration::from_secs(2));
    let source = MiniatureReader::open_existing(&case.source, case.known).unwrap();
    assert!(source.is_read_only().unwrap());
    let expected = source.snapshot_bytes().unwrap();
    drop(source);
    let before = product_tree(&case.root);
    assert!(!case.root.join("calibration.sqlite").exists());
    assert!(!case.root.join("fault.sqlite").exists());
    fs::create_dir(case.root.join("control")).unwrap();
    create(
        &case.root.join("control/protected-known.bin"),
        &worker::known_bytes(case.known),
    );
    create(&case.root.join("control/expected-state.bin"), &expected);

    let calibration = traced(&case, "calibration", deadline, epoch_deadline, None);
    assert_eq!(
        bounded(&calibration.control.join("result.txt"), 64),
        b"QUALIFIED_READER\n"
    );
    let calibrated = trace::final_sync(
        &calibration.trace,
        case.root.join("calibration.sqlite").to_str().unwrap(),
        "calibration.sqlite",
        calibration.writer,
        false,
    );
    assert!(
        calibration
            .stdout
            .windows(22)
            .any(|part| part == b"RESTORE_QUALIFIED_ACK\n")
    );
    let mut after_calibration = product_tree(&case.root);
    let calibration_bytes = match after_calibration
        .remove(Path::new("calibration.sqlite"))
        .unwrap()
    {
        Entry::File(bytes) => bytes,
        _ => panic!("retained calibration must be a standalone file"),
    };
    assert!((100..=1_048_576).contains(&calibration_bytes.len()));
    assert_eq!(
        after_calibration, before,
        "full original tree remains unchanged"
    );
    case.assert_original();
    let before_fault = product_tree(&case.root);

    let fault = traced(&case, "fault", deadline, epoch_deadline, Some(&calibrated));
    assert_eq!(
        bounded(&fault.control.join("result.txt"), 64),
        b"REFUSED_STORAGE_IO\n"
    );
    assert!(
        !fault
            .stdout
            .windows(22)
            .any(|part| part == b"RESTORE_QUALIFIED_ACK\n")
    );
    let failed = trace::final_sync(
        &fault.trace,
        case.root.join("fault.sqlite").to_str().unwrap(),
        "fault.sqlite",
        fault.writer,
        true,
    );
    assert_eq!(failed.syscall, calibrated.syscall);
    assert_eq!(
        failed.ordinal, calibrated.ordinal,
        "same actual per-thread syscall occurrence"
    );
    let mut after = product_tree(&case.root);
    let retained = match after.remove(Path::new("fault.sqlite")).unwrap() {
        Entry::File(bytes) => bytes,
        _ => panic!("failed output must remain retained and unqualified"),
    };
    assert!((100..=1_048_576).contains(&retained.len()));
    for suffix in ["-wal", "-shm", "-journal"] {
        if let Some(entry) = after.remove(&PathBuf::from(format!("fault.sqlite{suffix}"))) {
            assert!(
                matches!(entry, Entry::File(_)),
                "only closed target companion files may remain"
            );
        }
    }
    assert_eq!(
        after, before_fault,
        "every previous name and byte, including source and backup, survives"
    );
    case.assert_original();
    create(&case.root.join("control/qualified-case.txt"), format!(
        "actual EIO; no reader/ACK; retained unqualified output\ncalibration PID {} TID {} FD {} {} ordinal {}\nfault PID {} TID {} FD {} {} ordinal {}\nelapsed micros {}\n",
        calibration.main, calibrated.tid, calibrated.fd, calibrated.syscall, calibrated.ordinal,
        fault.main, failed.tid, failed.fd, failed.syscall, failed.ordinal, started.elapsed().as_micros(),
    ).as_bytes());
    clock(deadline);
}
