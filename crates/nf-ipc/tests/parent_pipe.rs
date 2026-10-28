use nf_identity::private_storage::PrivateVault;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

struct Fixture {
    root: PathBuf,
    child: Option<Child>,
    reader: Option<JoinHandle<()>>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn private_parent_pipe_closure_stops_owned_readonly_node_before_its_duration_and_removes_publication()
 {
    let root = std::env::temp_dir().join(format!("nf-parent-pipe-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut fixture = Fixture {
        root: root.clone(),
        child: None,
        reader: None,
    };
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault_path = root.join("vault");
    let vault = PrivateVault::create(&vault_path, &saves).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .arg("serve-parent")
        .arg(&vault_path)
        .arg(&saves)
        .arg("owned-diagnostic")
        .args([
            "01".repeat(16),
            "02".repeat(16),
            "03".repeat(32),
            "04".repeat(32),
            "05".repeat(16),
            "06".repeat(16),
            "60000".into(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    fixture.child = Some(command.spawn().unwrap());
    let child = fixture.child.as_mut().unwrap();
    let mut parent = child.stdin.take().unwrap();
    // Independent fixed protocol oracle, not an encoder imported from production.
    let request = [b"NF-PARENT-1\0".as_slice(), &[0x71; 32]].concat();
    let sent = parent.write_all(&request).and_then(|()| parent.flush());
    let mut output = child.stdout.take().unwrap();
    let (send, receive) = mpsc::sync_channel(1);
    fixture.reader = Some(std::thread::spawn(move || {
        let mut record = Zeroizing::new(vec![0; 263]);
        let result = output.read_exact(&mut record).map(|()| record);
        let _ = send.send(result);
    }));
    let record = receive
        .recv_timeout(Duration::from_secs(15))
        .expect("bounded private parent handoff")
        .expect("parent-owned descriptor handoff is missing");
    assert!(sent.is_ok(), "parent request was not accepted");
    assert_eq!(&record[..13], b"NF-HANDOFF-1\0");
    assert_eq!(&record[13..45], &[0x71; 32]);
    assert_eq!(&record[45..55], b"NF-IPC-R2\0");
    assert!(
        vault
            .read_optional_private_blob("owned-diagnostic")
            .unwrap()
            .is_some()
    );
    assert!(
        fixture
            .child
            .as_mut()
            .unwrap()
            .try_wait()
            .unwrap()
            .is_none()
    );
    drop(parent);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = fixture.child.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success(), "owned EOF shutdown failed");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "owned child did not honor parent EOF"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    fixture.child.take();
    assert!(
        vault
            .read_optional_private_blob("owned-diagnostic")
            .unwrap()
            .is_none()
    );
}
