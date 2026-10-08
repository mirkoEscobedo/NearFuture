use super::fixture_support::{self as f, Mode};
use crate::model::IdentityError;
use std::{
    fs,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use subc_jobobject::ContainedChild;
#[test]
fn drop_reaps_known_live_child_and_grandchild_before_inner_job_close() {
    let p = f::fixture(Mode::Descendant, None).unwrap();
    let h = f::held(&p);
    let ids = f::ready(&p);
    for id in &ids {
        f::observe(id, false)
    }
    drop(p);
    f::reaped(h);
    for id in &ids {
        f::observe(id, true)
    }
}
#[test]
fn actual_original_five_second_deadline_reaps_silent_helper() {
    let p = f::fixture(Mode::Silent, None).unwrap();
    let deadline = p.deadline;
    let h = f::held(&p);
    assert_eq!(
        p.request(b"GO\n".to_vec(), 1),
        Err(IdentityError::PrivateStorage)
    );
    assert!(Instant::now() >= deadline);
    drop(p);
    f::reaped(h);
}
#[test]
fn pipe_filling_stdout_and_inherited_handles_do_not_block_cleanup() {
    let p = f::fixture(Mode::FullPipes, None).unwrap();
    let h = f::held(&p);
    let ids = f::ready(&p);
    for id in &ids {
        f::observe(id, false)
    }
    p.send(Some(b"GO\n".to_vec())).unwrap();
    assert_eq!(p.line(), Err(IdentityError::PrivateStorage));
    drop(p);
    f::reaped(h);
    for id in &ids {
        f::observe(id, true)
    }
}
struct Owner(ContainedChild<Child>);
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.0.job.terminate();
        let _ = self.0.child.kill();
        let _ = self.0.child.wait();
    }
}
#[test]
fn abrupt_owner_death_reaps_known_descendants_before_caller_job_cleanup() {
    let scratch = f::Scratch::new();
    let ready = scratch.path().join("owner-ready");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "private_acl_session::process::fixture_lifecycle_tests::fixture_owner_process",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NF_ACL_ENGINE_OWNER_READY", &ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let started = Instant::now();
    let mut owner = Owner(subc_jobobject::spawn_contained(&mut command).unwrap());
    let deadline = started + Duration::from_secs(5);
    while !ready.exists() {
        assert!(owner.0.child.try_wait().unwrap().is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let ids = f::parse_ready(&fs::read(&ready).unwrap());
    for id in &ids {
        f::observe(id, false)
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "owner kill occurs before later-started adapter deadline"
    );
    owner.0.child.kill().unwrap();
    owner.0.child.wait().unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "actual owner exit precedes later-started adapter deadline"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while owner.0.job.process_count().unwrap() != 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    for id in &ids {
        f::observe(id, true)
    }
    assert_eq!(owner.0.job.process_count().unwrap(), 0);
    // Caller Job is still held. Its Drop has not terminated or closed it.
}
#[test]
fn fixture_owner_process() {
    let Some(ready) = std::env::var_os("NF_ACL_ENGINE_OWNER_READY") else {
        return;
    };
    let p = f::fixture(Mode::Descendant, None).unwrap();
    let _held = f::held(&p);
    let ids = f::ready(&p);
    let bytes = format!(
        "READY {} {} {} {}\n",
        ids[0].pid, ids[0].ticks, ids[1].pid, ids[1].ticks
    )
    .into_bytes();
    assert!(bytes.len() <= 64);
    let path = std::path::PathBuf::from(ready);
    let pending = path.with_extension("pending");
    fs::write(&pending, bytes).unwrap();
    fs::rename(pending, path).unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(100));
    }
}
