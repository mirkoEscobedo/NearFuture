mod chat_java_status_support;
use chat_java_status_support::Fixture;
use nf_ipc::DiscoveryRecord;
use nf_store::chat::{
    ChatStore, KnownChatFrontiers,
    outbox::{ClientOutbox, OutgoingState},
};
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct OwnedChild(Option<Child>);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.stdin(Stdio::null());
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn physical(
    f: &Fixture,
    java: &Path,
    classpath: &str,
    name: &str,
    known: KnownChatFrontiers,
    revision: u64,
    phase: u8,
    receipt: &[u8],
) {
    let config = f.config(1);
    let mut command = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    hidden(&mut command);
    command
        .arg("serve-chat")
        .arg(&f.vault_path)
        .arg(&f.saves_root)
        .arg(name)
        .arg(&f.store_path)
        .arg(&f.outbox_path)
        .args([
            hex(&config.universe),
            hex(&config.history),
            hex(&config.ruleset),
            hex(&config.content_policy),
            hex(&f.alice.public.peer),
            known.revision.to_string(),
            known.membership_revision.to_string(),
            revision.to_string(),
            hex(f.bob.public.account.as_bytes()),
            hex(f.bob.public.device.as_bytes()),
            "12000".into(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut node = OwnedChild(Some(command.spawn().unwrap()));
    let ready = Instant::now() + Duration::from_secs(4);
    let record = loop {
        assert!(
            node.0.as_mut().unwrap().try_wait().unwrap().is_none(),
            "production node exited before publication"
        );
        if let Ok(record) = DiscoveryRecord::read(&f.vault, name) {
            break record;
        }
        assert!(
            Instant::now() < ready,
            "production Chat mount did not publish"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(record.principal().account, f.alice.public.account);
    assert_eq!(record.principal().device, f.alice.public.device);
    assert_eq!(record.config().universe, config.universe);
    assert_eq!(record.config().history, config.history);
    assert_ne!(record.config().runtime_session, 0);
    let mut command = Command::new(java);
    hidden(&mut command);
    command
        .args(["-cp", classpath, "nf.adapter.ipc.ChatStatusSocketClient"])
        .arg(f.vault_path.join(format!("blob-{name}")))
        .args([
            hex(&[111; 16]),
            hex(&[101; 16]),
            hex(&[81; 16]),
            phase.to_string(),
            revision.to_string(),
            hex(&f.canonical_signed()),
            if receipt.is_empty() {
                "-".into()
            } else {
                hex(receipt)
            },
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = OwnedChild(Some(command.spawn().unwrap()));
    let until = Instant::now() + Duration::from_secs(6);
    loop {
        if client.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < until,
            "owned Java client exceeded deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = client.0.take().unwrap().wait_with_output().unwrap();
    // Business RED qualifies only if the actual Java projection assertion is reached after mutual auth.
    assert!(
        output.status.success(),
        "Java Chat first failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "CHAT_STATUS_SOCKET_OK"
    );
    assert!(output.stderr.is_empty());
    let until = Instant::now() + Duration::from_secs(13);
    loop {
        if let Some(status) = node.0.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(
            Instant::now() < until,
            "bounded node failed normal shutdown"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    node.0.take();
    assert!(!f.vault_path.join(format!("blob-{name}")).exists());
}

/// Explicit Java dependency: absence fails when this physical case is selected.
#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn java_production_consumer_reports_pending_then_genuine_delivered_original_through_chat_cli() {
    let java = std::env::var_os("NF_IPC_JAVA").expect("NF_IPC_JAVA required");
    let classpath_file =
        std::env::var_os("NF_IPC_JAVA_CLASSPATH_FILE").expect("classpath required");
    let classpath = fs::read_to_string(classpath_file).unwrap();
    assert!(!classpath.trim().is_empty() && classpath.len() < 65536);
    let f = Fixture::new();
    f.create_pending();
    let initial_store = fs::read(&f.store_path).unwrap();
    let initial_outbox = fs::read(&f.outbox_path).unwrap();
    physical(
        &f,
        Path::new(&java),
        classpath.trim(),
        "chat-java-pending",
        f.initial_known(),
        1,
        1,
        &[],
    );
    assert_eq!(fs::read(&f.store_path).unwrap(), initial_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), initial_outbox);
    let mut store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    let delivered = f.deliver_locally(&mut store, &mut outbox);
    assert!(matches!(&delivered.state, OutgoingState::Delivered(_)));
    assert_eq!(delivered.signed, f.signed);
    assert_eq!(delivered.original_request, f.request());
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 1);
    assert_eq!(outbox.known_revision(), Ok(2));
    drop(outbox);
    drop(store);
    let final_store = fs::read(&f.store_path).unwrap();
    let final_outbox = fs::read(&f.outbox_path).unwrap();
    physical(
        &f,
        Path::new(&java),
        classpath.trim(),
        "chat-java-delivered",
        known,
        2,
        2,
        &f.expected_receipt(),
    );
    assert_eq!(fs::read(&f.store_path).unwrap(), final_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), final_outbox);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(store.current_membership().unwrap(), f.state);
    assert_eq!(outbox.entry([81; 16]).unwrap(), Some(delivered));
    assert_eq!(outbox.known_revision(), Ok(2));
    drop(outbox);
    drop(store);
    assert_eq!(fs::read(&f.store_path).unwrap(), final_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), final_outbox);
}
