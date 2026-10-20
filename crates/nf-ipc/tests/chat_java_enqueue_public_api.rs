mod chat_java_enqueue_support;
use chat_java_enqueue_support::{Fixture, hash, message_bytes};
use nf_contract::identity::RequestId;
use nf_ipc::{ChatCommandPort, DiscoveryRecord};
use nf_store::chat::{
    ChatStore, SignedMessage,
    codec::decode_signed_message,
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
    revision: u64,
    attempt: u8,
) -> Vec<u8> {
    let config = f.config(1);
    let mut command = Command::new(env!("CARGO_BIN_EXE_nf-ipc-node"));
    hidden(&mut command);
    command
        .arg("serve-chat-command")
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
            "0".into(),
            "1".into(),
            revision.to_string(),
            hex(f.bob.public.account.as_bytes()),
            hex(f.bob.public.device.as_bytes()),
            "12000".into(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut node = OwnedChild(Some(command.spawn().unwrap()));
    let ready = Instant::now() + Duration::from_secs(4);
    let record = loop {
        if node.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            let output = node.0.take().unwrap().wait_with_output().unwrap();
            assert!(output.stdout.len() <= 4096 && output.stderr.len() <= 4096);
            assert!(
                output.status.success(),
                "existing command-owner setup failed: exit={:?}; error={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("successful command owner exited without publication");
        }
        if let Ok(record) = DiscoveryRecord::read(&f.vault, name) {
            break record;
        }
        assert!(
            Instant::now() < ready,
            "command-owner publication deadline exceeded"
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
        .args([
            "-cp",
            classpath,
            "nf.adapter.ipc.ChatEnqueuePublicApiSocketClient",
        ])
        .arg(f.vault_path.join(format!("blob-{name}")))
        .args([
            hex(&[attempt; 16]),
            hex(&[101; 16]),
            "Public Java enqueue original".into(),
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
            "owned Java enqueue client exceeded deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = client.0.take().unwrap().wait_with_output().unwrap();
    assert!(output.stdout.len() <= 8192 && output.stderr.len() <= 4096);
    assert!(
        output.status.success(),
        "Java enqueue contract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    let signed_hex = text
        .trim()
        .strip_prefix("CHAT_ENQUEUE_PUBLIC_API_OK ")
        .expect("public enqueue result marker");
    assert!(
        signed_hex.is_ascii()
            && signed_hex.len().is_multiple_of(2)
            && (348..=4442).contains(&signed_hex.len())
    );
    let signed = (0..signed_hex.len() / 2)
        .map(|i| u8::from_str_radix(&signed_hex[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    let until = Instant::now() + Duration::from_secs(13);
    loop {
        if let Some(status) = node.0.as_mut().unwrap().try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(
            Instant::now() < until,
            "bounded command owner failed normal shutdown"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = node.0.take().unwrap().wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .replace("\r\n", "\n")
            .trim(),
        "READY\nSTOPPED"
    );
    assert!(output.stderr.is_empty());
    assert!(!f.vault_path.join(format!("blob-{name}")).exists());
    signed
}
fn verify_original(f: &Fixture, bytes: &[u8]) -> SignedMessage {
    let signed = decode_signed_message(bytes).unwrap();
    assert_eq!(signed.message.scope, f.policy.scope);
    assert_eq!(signed.message.channel, nf_store::chat::Channel::General);
    assert_eq!(signed.message.author.account, f.alice.public.account);
    assert_eq!(signed.message.author.device, f.alice.public.device);
    assert_eq!(signed.message.sequence, 1);
    assert_eq!(signed.message.text, "Public Java enqueue original");
    assert_ne!(signed.message.message, [0; 16]);
    let independent = message_bytes(&signed.message);
    assert_eq!(
        nf_contract::signatures::verify_digest(
            &f.alice.public.device_key,
            &hash(&independent),
            &signed.signature
        ),
        Ok(())
    );
    let mut canonical = independent;
    canonical.extend_from_slice(&signed.signature);
    assert_eq!(canonical, bytes);
    signed
}
/// Missing Java/executable/classpath dependencies fail when explicitly selected; never substitute a mock session.
#[test]
#[ignore = "requires NF_IPC_JAVA and NF_IPC_JAVA_CLASSPATH_FILE; exportAuthTestClasspath first"]
fn public_java_enqueue_api_signs_one_original_then_reopens_and_exactly_retries() {
    let whole = Instant::now() + Duration::from_secs(40);
    let java = std::env::var_os("NF_IPC_JAVA").expect("NF_IPC_JAVA required");
    let classpath_file =
        std::env::var_os("NF_IPC_JAVA_CLASSPATH_FILE").expect("classpath required");
    let classpath = fs::read_to_string(classpath_file).unwrap();
    assert!(!classpath.trim().is_empty() && classpath.len() < 65536);
    let f = Fixture::new();
    // Independently prove the real owner inputs before exercising the public Java API.
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    assert_eq!(store.current_membership().unwrap(), f.state);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    let port = ChatCommandPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1)
        .unwrap();
    assert_eq!(port.principal().account, f.alice.public.account);
    assert_eq!(port.principal().device, f.alice.public.device);
    drop(port);
    let store_before = fs::read(&f.store_path).unwrap();
    let bytes = physical(
        &f,
        Path::new(&java),
        classpath.trim(),
        "chat-java-public-api-first",
        0,
        111,
    );
    let signed = verify_original(&f, &bytes);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    let original = outbox.entry(signed.message.message).unwrap().unwrap();
    assert_eq!(original.original_request, RequestId::from_bytes([101; 16]));
    assert_eq!(original.signed, signed);
    assert_eq!(original.state, OutgoingState::Pending);
    assert_eq!(outbox.known_revision(), Ok(1));
    drop(outbox);
    let outbox_before = fs::read(&f.outbox_path).unwrap();
    let retried = physical(
        &f,
        Path::new(&java),
        classpath.trim(),
        "chat-java-public-api-retry",
        1,
        112,
    );
    assert_eq!(retried, bytes);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), f.initial_known());
    assert_eq!(store.current_membership().unwrap(), f.state);
    assert_eq!(
        outbox.entry(signed.message.message).unwrap(),
        Some(original)
    );
    assert_eq!(outbox.known_revision(), Ok(1));
    drop(outbox);
    drop(store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    assert!(Instant::now() < whole);
}
