#![cfg(target_os = "linux")]
mod support;
use nf_store::{KnownFrontiers, RequestStatus, Store, StoreError};
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct OwnedTracer {
    child: Child,
    reaped: bool,
}
impl Drop for OwnedTracer {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
#[test]
fn sync_fault_worker() {
    let Some(path) = std::env::var_os("NF_STORE_SYNC_DATABASE") else {
        return;
    };
    let world = support::strategic_world();
    let spec = world.to_spec();
    let mut store =
        Store::open_existing(&path, KnownFrontiers::genesis(spec.universe, spec.history)).unwrap();
    let (_, batch) = support::batch(store.world(), store.pending().unwrap());
    assert_eq!(store.commit(&batch), Err(StoreError::Io));
    assert_eq!(
        store.query(&support::intent(&world, 7, -25), support::device()),
        Err(StoreError::Quarantined)
    );
    std::fs::write(
        std::env::var_os("NF_STORE_SYNC_RESULT").unwrap(),
        b"commit-refused-no-ack",
    )
    .unwrap();
}
#[test]
fn real_linux_sync_failure_refuses_commit_and_recovers_pending_binding() {
    if std::env::var_os("NF_STORE_REQUIRE_SYNC_FAULT").as_deref() != Some(std::ffi::OsStr::new("1"))
    {
        eprintln!(
            "UNOBSERVED: set NF_STORE_REQUIRE_SYNC_FAULT=1 with maintained strace installed to require the real sync gate"
        );
        return;
    }
    let scratch = support::Scratch::new();
    let world = support::strategic_world();
    let intent = support::intent(&world, 7, -25);
    let frontier = support::frontier(&world, &intent);
    let mut store = Store::create(scratch.db(), &world).unwrap();
    store
        .prepare(&frontier, &support::devices(&intent), &[])
        .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let trace = scratch.0.join("sync.trace");
    let result = scratch.0.join("sync.result");
    let child = Command::new("strace")
        .args([
            "--kill-on-exit",
            "-f",
            "-yy",
            "-e",
            "trace=fsync,fdatasync",
            "-e",
            "inject=fsync,fdatasync:error=EIO",
            "-o",
        ])
        .arg(&trace)
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sync_fault_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NF_STORE_SYNC_DATABASE", scratch.db())
        .env("NF_STORE_SYNC_RESULT", &result)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("required maintained strace injector unavailable");
    let mut child = OwnedTracer {
        child,
        reaped: false,
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.child.try_wait().unwrap() {
            child.reaped = true;
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "owned tracer timed out; Drop kills/reaps and strace EXITKILL contains the exact child"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success(), "required actual sync fault worker failed");
    assert!(
        std::fs::metadata(&trace).unwrap().len() <= 1_048_576,
        "bounded trace"
    );
    let trace = std::fs::read_to_string(&trace).unwrap();
    assert!(
        trace.lines().any(
            |line| (line.contains("fsync(") || line.contains("fdatasync("))
                && line.contains("history.sqlite")
                && line.contains("EIO")
                && line.contains("INJECTED")
        ),
        "must observe actual SQLite file sync refusal"
    );
    assert_eq!(std::fs::read(&result).unwrap(), b"commit-refused-no-ack");
    let store = Store::open_existing(scratch.db(), known).unwrap();
    assert_eq!(store.world(), &world);
    assert_eq!(store.outbox().count(), 0);
    assert!(matches!(
        store.query(&intent, support::device()).unwrap(),
        Some(RequestStatus::Pending { .. })
    ));
}
