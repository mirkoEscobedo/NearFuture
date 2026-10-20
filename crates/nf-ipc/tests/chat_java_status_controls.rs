#[path = "chat_java_status_support/mod.rs"]
mod chat_java_status_support;
use chat_java_status_support::Fixture;
use nf_identity::{model::DeviceRevocation, rotation::revocation_digest};
use nf_ipc::DiscoveryRecord;
use nf_store::chat::{ChatStore, KnownChatFrontiers, outbox::ClientOutbox};
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
fn hidden(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c.stdin(Stdio::null());
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn command(
    f: &Fixture,
    name: &str,
    known: KnownChatFrontiers,
    revision: u64,
    peer: &[u8],
) -> Command {
    let config = f.config(1);
    let mut c = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    hidden(&mut c);
    c.arg("serve-chat")
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
            hex(peer),
            known.revision.to_string(),
            known.membership_revision.to_string(),
            revision.to_string(),
            hex(f.bob.public.account.as_bytes()),
            hex(f.bob.public.device.as_bytes()),
            "12000".into(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    c
}
fn physical(
    f: &Fixture,
    name: &str,
    known: KnownChatFrontiers,
    revision: u64,
    mode: &str,
    receipt: &[u8],
) {
    let java = std::env::var_os("NF_IPC_JAVA").expect("NF_IPC_JAVA required");
    let cp = fs::read_to_string(
        std::env::var_os("NF_IPC_JAVA_CLASSPATH_FILE").expect("classpath required"),
    )
    .unwrap();
    assert!(!cp.trim().is_empty() && cp.len() < 65536);
    let before_store = fs::read(&f.store_path).unwrap();
    let before_outbox = fs::read(&f.outbox_path).unwrap();
    let mut node = OwnedChild(Some(
        command(f, name, known, revision, &f.alice.public.peer)
            .spawn()
            .unwrap(),
    ));
    let ready = Instant::now() + Duration::from_secs(4);
    let record = loop {
        assert!(
            node.0.as_mut().unwrap().try_wait().unwrap().is_none(),
            "CLI setup exited before publication"
        );
        if let Ok(record) = DiscoveryRecord::read(&f.vault, name) {
            break record;
        }
        assert!(Instant::now() < ready, "CLI setup did not publish");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(record.principal().account, f.alice.public.account);
    assert_eq!(record.principal().device, f.alice.public.device);
    assert_ne!(record.config().runtime_session, 0);
    let mut c = Command::new(Path::new(&java));
    hidden(&mut c);
    c.args([
        "-cp",
        cp.trim(),
        "nf.adapter.ipc.ChatStatusControlSocketClient",
    ])
    .arg(f.vault_path.join(format!("blob-{name}")))
    .args([
        mode.to_owned(),
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
    let mut client = OwnedChild(Some(c.spawn().unwrap()));
    let end = Instant::now() + Duration::from_secs(6);
    loop {
        if client.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < end,
            "owned Java controls exceeded deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = client.0.take().unwrap().wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Java Chat control failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .any(|x| x == "CHAT_STATUS_CONTROLS_OK")
    );
    assert!(output.stderr.is_empty());
    let end = Instant::now() + Duration::from_secs(13);
    loop {
        if let Some(status) = node.0.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < end, "node failed bounded normal shutdown");
        std::thread::sleep(Duration::from_millis(5));
    }
    node.0.take();
    assert!(!f.vault_path.join(format!("blob-{name}")).exists());
    assert_eq!(fs::read(&f.store_path).unwrap(), before_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), before_outbox);
}
#[test]
#[ignore = "requires actual NF_IPC_JAVA and freshly exported Java classpath"]
fn java_chat_consumer_refuses_closed_profile_canonical_correlation_and_lifecycle_inputs() {
    let f = Fixture::new();
    f.create_pending();
    physical(
        &f,
        "chat-java-refusals-pending",
        f.initial_known(),
        1,
        "refusals",
        &[],
    );
    let mut store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    // One legitimate public setup post/receipt/ACK, never an adversarial fixture response or SQL seed.
    let delivered = f.deliver_locally(&mut store, &mut outbox);
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 1);
    assert_eq!(outbox.known_revision(), Ok(2));
    drop(outbox);
    drop(store);
    physical(
        &f,
        "chat-java-refusals-delivered",
        known,
        2,
        "refusals",
        &f.expected_receipt(),
    );
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(store.current_membership().unwrap(), f.state);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
    assert_eq!(outbox.entry([81; 16]).unwrap(), Some(delivered));
    assert_eq!(outbox.known_revision(), Ok(2));
}
fn budget(mode: &str) {
    let f = Fixture::new();
    f.create_pending();
    physical(
        &f,
        &format!("chat-java-budget-{mode}"),
        f.initial_known(),
        1,
        mode,
        &[],
    );
}
#[test]
#[ignore = "requires actual Java mutual authentication; independent negotiated-depth RED"]
fn java_chat_consumer_refuses_under_negotiated_nesting_depth() {
    budget("depth");
}
#[test]
#[ignore = "requires actual Java mutual authentication; independent negotiated-items RED"]
fn java_chat_consumer_refuses_over_negotiated_collection_items_before_profile_admission() {
    budget("items");
}
#[test]
#[ignore = "requires actual Java mutual authentication; independent negotiated-decoded-budget RED"]
fn java_chat_consumer_refuses_under_negotiated_decoded_allocation_budget() {
    budget("decoded");
}
fn refused_mount(f: &Fixture, name: &str, known: KnownChatFrontiers, peer: &[u8]) {
    let before_store = fs::read(&f.store_path).unwrap();
    let before_outbox = fs::read(&f.outbox_path).unwrap();
    let mut node = OwnedChild(Some(command(f, name, known, 1, peer).spawn().unwrap()));
    let end = Instant::now() + Duration::from_secs(4);
    loop {
        assert!(
            DiscoveryRecord::read(&f.vault, name).is_err(),
            "invalid authority must not publish"
        );
        if let Some(status) = node.0.as_mut().unwrap().try_wait().unwrap() {
            assert!(!status.success());
            break;
        }
        assert!(
            Instant::now() < end,
            "invalid CLI failed to refuse before deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    node.0.take();
    assert!(!f.vault_path.join(format!("blob-{name}")).exists());
    assert_eq!(fs::read(&f.store_path).unwrap(), before_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), before_outbox);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    assert_eq!(outbox.entry([81; 16]).unwrap(), Some(f.pending()));
    assert_eq!(outbox.known_revision(), Ok(1));
}
#[test]
fn chat_cli_refuses_wrong_retained_peer_before_publication_without_sql_changes() {
    let f = Fixture::new();
    f.create_pending();
    refused_mount(&f, "chat-java-wrong-peer", f.initial_known(), &[43; 32]);
}
#[test]
fn chat_cli_refuses_publicly_revoked_receiver_before_publication_without_sql_changes() {
    let f = Fixture::new();
    f.create_pending();
    let mut store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let change = DeviceRevocation {
        scope: f.policy.scope,
        issuer: f.alice.public.account,
        device: f.bob.public.device,
        frontier: 1,
    };
    let signature = f.alice.account_key.sign(&revocation_digest(&change));
    let revoked = store.revoke_device(&change, &signature).unwrap();
    assert_eq!(revoked.revision, 2);
    assert!(revoked.devices.get(&f.bob.public.device).unwrap().revoked);
    let known = store.known_frontiers().unwrap();
    drop(store);
    refused_mount(
        &f,
        "chat-java-revoked-receiver",
        known,
        &f.alice.public.peer,
    );
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    assert_eq!(store.current_membership().unwrap(), revoked);
}
