mod support;
use nf_store::{KnownFrontiers, RequestStatus, Store};
use std::{
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn block_at(marker: &std::path::Path) {
    let mut file = std::fs::File::create(marker).unwrap();
    use std::io::Write;
    file.write_all(b"ready").unwrap();
    file.sync_all().unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    std::process::exit(91)
}
#[test]
fn crash_child_worker() {
    let Ok(path) = std::env::var("NF_STORE_CRASH_DATABASE") else {
        return;
    };
    let target = std::env::var("NF_STORE_CRASH_POINT").unwrap();
    let marker = std::path::PathBuf::from(std::env::var("NF_STORE_CRASH_READY").unwrap());
    let spec = support::strategic_world().to_spec();
    let mut store =
        Store::open_existing(path, KnownFrontiers::genesis(spec.universe, spec.history)).unwrap();
    let (_, batch) = support::batch(store.world(), store.pending().unwrap());
    store
        .commit_with_hook(&batch, &mut |point| {
            if format!("{point:?}") == target {
                block_at(&marker);
            }
            Ok(())
        })
        .unwrap();
    let ack = marker.with_extension("ack");
    std::fs::write(ack, b"acknowledged").unwrap();
    if target == "AfterAcknowledgement" {
        block_at(&marker);
    }
}
#[test]
fn killed_processes_preserve_atomicity_before_and_after_commit_and_acknowledgement() {
    for point in [
        "BeforeTransaction",
        "AfterWrites",
        "BeforeCommit",
        "AfterCommit",
        "BeforeAcknowledgement",
        "AfterAcknowledgement",
    ] {
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
        let ready = scratch.0.join("ready");
        let mut child = OwnedChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "crash_child_worker",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("NF_STORE_CRASH_DATABASE", scratch.db())
                .env("NF_STORE_CRASH_POINT", point)
                .env("NF_STORE_CRASH_READY", &ready)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                Instant::now() < deadline,
                "Child boundary deadline: {point}"
            );
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "Child exited before {point}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        child.0.kill().unwrap();
        let status = child.0.wait().unwrap();
        assert!(!status.success());
        assert!(child.0.try_wait().unwrap().is_some());
        let store = Store::open_existing(scratch.db(), known).unwrap();
        let committed = matches!(
            point,
            "AfterCommit" | "BeforeAcknowledgement" | "AfterAcknowledgement"
        );
        if committed {
            assert!(matches!(
                store.query(&intent, support::device()).unwrap(),
                Some(RequestStatus::Committed { .. })
            ));
            assert_eq!(
                store
                    .world()
                    .view()
                    .market(nf_contract::identity::EntityId::from_bytes([30; 16]))
                    .unwrap()
                    .credits,
                75
            );
            assert_eq!(store.outbox().count(), 1);
        } else {
            assert_eq!(
                store.query(&intent, support::device()).unwrap(),
                Some(RequestStatus::Pending {
                    operation: intent.operation
                })
            );
            assert_eq!(store.world(), &world);
            assert_eq!(store.outbox().count(), 0);
        }
        assert_eq!(
            ready.with_extension("ack").exists(),
            point == "AfterAcknowledgement"
        );
    }
}
