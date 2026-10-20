mod common;
use common::OwnedOutput;
use nf_identity::private_storage::PrivateVault;
use nf_ipc::DiscoveryRecord;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Fixture {
    root: PathBuf,
    child: Option<Child>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn command() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c.stdin(Stdio::null());
    c
}
#[test]
fn foreground_node_discovers_attaches_queries_readonly_and_restarts_with_fresh_session() {
    let root = std::env::temp_dir().join(format!("nf-ipc-process-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut fixture = Fixture {
        root: root.clone(),
        child: None,
    };
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault_path = root.join("vault");
    let vault = PrivateVault::create_detailed(&vault_path, &saves).unwrap();
    let mut old_session = None;
    for iteration in 0..2 {
        let name = format!("ipc-run-{iteration}");
        let mut child = command();
        child
            .arg("serve")
            .arg(&vault_path)
            .arg(&saves)
            .arg(&name)
            .args([
                "01".repeat(16),
                "02".repeat(16),
                "03".repeat(32),
                "04".repeat(32),
                "05".repeat(16),
                "06".repeat(16),
                "6000".into(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        fixture.child = Some(child.spawn().unwrap());
        let start = Instant::now();
        let record = loop {
            assert!(
                fixture
                    .child
                    .as_mut()
                    .unwrap()
                    .try_wait()
                    .unwrap()
                    .is_none()
            );
            if vault_path.join(format!("blob-{name}")).exists()
                && let Ok(record) = DiscoveryRecord::read(&vault, &name)
            {
                break record;
            }
            assert!(start.elapsed() < Duration::from_secs(15));
            std::thread::sleep(Duration::from_millis(10));
        };
        if let Some(old) = old_session {
            assert_ne!(old, record.config().runtime_session);
        }
        old_session = Some(record.config().runtime_session);
        let attached = command()
            .arg("attach")
            .arg(&vault_path)
            .arg(&saves)
            .arg(&name)
            .bounded_output();
        assert!(attached.status.success(), "{:?}", attached.status);
        assert_eq!(
            String::from_utf8(attached.stdout).unwrap().trim(),
            "ATTACHED protocol 1 (mutual local token scope)"
        );
        if iteration == 0 {
            let query = command()
                .arg("query")
                .arg(&vault_path)
                .arg(&saves)
                .arg(&name)
                .arg("08".repeat(16))
                .bounded_output();
            assert!(!query.status.success());
            assert_eq!(
                String::from_utf8(query.stderr).unwrap().trim(),
                "IPC unavailable: Unsupported"
            );
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = fixture.child.as_mut().unwrap().try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        fixture.child.take();
        assert!(!vault_path.join(format!("blob-{name}")).exists());
        assert!(
            command()
                .arg("attach")
                .arg(&vault_path)
                .arg(&saves)
                .arg(&name)
                .bounded_output()
                .status
                .code()
                .is_some_and(|v| v != 0)
        );
    }
}
