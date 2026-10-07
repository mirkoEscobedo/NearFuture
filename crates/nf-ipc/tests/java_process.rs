use nf_identity::private_storage::PrivateVault;
use nf_ipc::DiscoveryRecord;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Owned {
    root: PathBuf,
    children: Vec<Child>,
}
impl Drop for Owned {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn hidden(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c.stdin(Stdio::null());
}
/// Explicit opt-in: absence of the independently built Java classpath/executable is a failure.
#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_control_and_bulk_mutual_authenticate_with_owned_foreground_rust_node() {
    let java = std::env::var_os("NF_IPC_JAVA").expect("NF_IPC_JAVA required");
    let classpath_file =
        std::env::var_os("NF_IPC_JAVA_CLASSPATH_FILE").expect("classpath file required");
    let classpath = fs::read_to_string(classpath_file).unwrap();
    assert!(!classpath.trim().is_empty() && classpath.len() < 65536);
    let root = std::env::temp_dir().join(format!("nf-ipc-java-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let mut owned = Owned {
        root: root.clone(),
        children: vec![],
    };
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault_path = root.join("vault");
    let vault = PrivateVault::create(&vault_path, &saves).unwrap();
    let mut node = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    hidden(&mut node);
    node.arg("serve")
        .arg(&vault_path)
        .arg(&saves)
        .arg("java-parity")
        .args([
            "01".repeat(16),
            "02".repeat(16),
            "03".repeat(32),
            "04".repeat(32),
            "05".repeat(16),
            "06".repeat(16),
            "12000".into(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    owned.children.push(node.spawn().unwrap());
    let ready = Instant::now() + Duration::from_secs(15);
    loop {
        assert!(owned.children[0].try_wait().unwrap().is_none());
        if DiscoveryRecord::read(&vault, "java-parity").is_ok() {
            break;
        }
        assert!(Instant::now() < ready);
        std::thread::sleep(Duration::from_millis(10));
    }
    for role in ["control", "bulk"] {
        let mut client = Command::new(&java);
        hidden(&mut client);
        client
            .args(["-cp", classpath.trim(), "nf.adapter.ipc.AuthSocketClient"])
            .arg(vault_path.join("blob-java-parity"))
            .arg(role)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        owned.children.push(client.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(8);
        let index = owned.children.len() - 1;
        loop {
            if owned.children[index].try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "owned Java client exceeded deadline"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let result = owned.children.pop().unwrap().wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "Java auth failed: {:?}",
            result.status
        );
        assert_eq!(
            String::from_utf8(result.stdout).unwrap().trim(),
            "AUTH_SOCKET_OK"
        );
        assert!(result.stderr.is_empty());
    }
    // Node is intentionally bounded; normal shutdown removes the exact owned publication.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = owned.children[0].try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    owned.children.pop();
    assert!(!vault_path.join("blob-java-parity").exists());
}
