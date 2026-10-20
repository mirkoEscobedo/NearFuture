use nf_identity::private_storage::PrivateVault;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Output, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

struct Owner {
    root: PathBuf,
    child: Option<Child>,
    reader: Option<JoinHandle<()>>,
}
impl Drop for Owner {
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
impl Owner {
    fn start(case: &str, duration: &str) -> (Self, ChildStdin, PrivateVault) {
        let root =
            std::env::temp_dir().join(format!("nf-parent-negative-{}-{case}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let mut owner = Self {
            root: root.clone(),
            child: None,
            reader: None,
        };
        let saves = root.join("saves");
        fs::create_dir(&saves).unwrap();
        let vault_path = root.join("vault");
        let vault = PrivateVault::create_diagnosed(&vault_path, &saves)
            .unwrap_or_else(|failure| panic!("private vault create failed: {failure:?}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        command
            .arg("serve-parent")
            .arg(vault_path)
            .arg(saves)
            .arg("negative-owner")
            .args([
                "01".repeat(16),
                "02".repeat(16),
                "03".repeat(32),
                "04".repeat(32),
                "05".repeat(16),
                "06".repeat(16),
                duration.into(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        owner.child = Some(command.spawn().unwrap());
        let input = owner.child.as_mut().unwrap().stdin.take().unwrap();
        (owner, input, vault)
    }
    fn handoff(&mut self, input: &mut ChildStdin) {
        input
            .write_all(&[b"NF-PARENT-1\0".as_slice(), &[0x71; 32]].concat())
            .unwrap();
        input.flush().unwrap();
        let mut output = self.child.as_mut().unwrap().stdout.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        self.reader = Some(std::thread::spawn(move || {
            let mut record = Zeroizing::new(vec![0; 263]);
            let result = output.read_exact(&mut record).map(|()| record);
            let _ = send.send(result);
        }));
        let record = receive
            .recv_timeout(Duration::from_secs(15))
            .expect("bounded handoff")
            .unwrap();
        assert_eq!(&record[..13], b"NF-HANDOFF-1\0");
        assert_eq!(&record[13..45], &[0x71; 32]);
        assert_eq!(&record[45..55], b"NF-IPC-R2\0");
    }
    fn finish(&mut self) -> Output {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.child.as_mut().unwrap().try_wait().unwrap().is_none() {
            assert!(
                Instant::now() < deadline,
                "owned negative process exceeded exit bound"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        self.child.take().unwrap().wait_with_output().unwrap()
    }
}

fn refused(case: &str, request: &[u8], eof: bool, reason: &str) {
    let (mut owner, mut parent, vault) = Owner::start(case, "60000");
    parent.write_all(request).unwrap();
    parent.flush().unwrap();
    let mut parent = Some(parent);
    if eof {
        drop(parent.take());
    }
    let output = owner.finish();
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "refused parent must not receive private output"
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!("IPC unavailable: {reason}\n")
    );
    assert!(
        vault
            .read_optional_private_blob("negative-owner")
            .unwrap()
            .is_none()
    );
}

#[test]
fn malformed_parent_magic_refuses_without_publication_or_private_output() {
    refused(
        "magic",
        &[b"NF-PARENT-X\0".as_slice(), &[0x71; 32]].concat(),
        false,
        "Malformed",
    );
}
#[test]
fn zero_parent_challenge_refuses_without_publication_or_private_output() {
    refused(
        "zero",
        &[b"NF-PARENT-1\0".as_slice(), &[0; 32]].concat(),
        false,
        "Malformed",
    );
}
#[test]
fn partial_parent_challenge_eof_refuses_without_publication_or_private_output() {
    refused("partial-eof", b"NF-PARENT-1\0\x71", true, "Malformed");
}
#[test]
fn partial_parent_challenge_with_open_pipe_expires_without_publication_or_private_output() {
    refused("partial-open", b"NF-PARENT-1\0\x71", false, "Disconnected");
}
#[test]
fn finite_duration_expires_with_parent_still_open_and_removes_publication() {
    let (mut owner, mut parent, vault) = Owner::start("expiry", "500");
    owner.handoff(&mut parent);
    let output = owner.finish();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(
        vault
            .read_optional_private_blob("negative-owner")
            .unwrap()
            .is_none()
    );
    drop(parent);
}
#[test]
fn extra_parent_input_revokes_the_owned_session_and_removes_publication() {
    let (mut owner, mut parent, vault) = Owner::start("extra", "60000");
    owner.handoff(&mut parent);
    parent.write_all(b"x").unwrap();
    parent.flush().unwrap();
    let output = owner.finish();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(
        vault
            .read_optional_private_blob("negative-owner")
            .unwrap()
            .is_none()
    );
}
