mod chat_local_processes_support;
#[path = "chat_peer_delivery_support/mod.rs"]
#[expect(
    dead_code,
    reason = "Shared delivery fixture provides additional oracles for other delivery tests"
)]
mod fixture;

use chat_local_processes_support::{Child, run_worker};
use fixture::{Fixture, author};
use nf_contract::identity::RequestId;
use nf_store::chat::{
    ChatStore, KnownChatFrontiers, codec,
    outbox::{ClientOutbox, OutboxProfile, OutgoingState},
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

#[test]
#[ignore = "management dispatcher: invoked only by the owning local process scenario"]
fn owned_chat_process_worker() {
    run_worker();
}

#[test]
fn two_local_client_processes_deliver_and_reconnect_to_owned_chat_sidecars() {
    let whole = Instant::now();
    let fixture = Fixture::new();
    let root = fixture.receiver_path.parent().unwrap();
    fs::write(
        root.join("membership.bin"),
        nf_identity::codec::encode_state(&fixture.state).unwrap(),
    )
    .unwrap();
    let evidence = PathBuf::from(
        std::env::var_os("NF_CHAT_COMMAND_QUERY_EVIDENCE_DIR")
            .expect("caller-owned public process evidence parent"),
    )
    .canonicalize()
    .unwrap()
    .join("local-two-client-process-flow");
    fs::create_dir(&evidence).expect("fresh public process evidence leaf");
    assert!(!evidence.starts_with(root));
    for (who, local, remote, id) in [
        ("alice", &fixture.alice, &fixture.bob, 81),
        ("bob", &fixture.bob, &fixture.alice, 82),
    ] {
        let store = ChatStore::create(
            root.join(format!("receiver-{who}.sqlite")),
            &fixture.policy,
            &fixture.state,
        )
        .unwrap();
        assert_eq!(store.known_frontiers().unwrap().revision, 0);
        drop(store);
        let profile = OutboxProfile::from_current(
            &fixture.policy,
            &fixture.state,
            author(local),
            author(remote),
        )
        .unwrap();
        let mut outbox =
            ClientOutbox::create(root.join(format!("sender-{who}.sqlite")), &profile).unwrap();
        let mut signed = fixture.signed.clone();
        signed.message.author = author(local);
        signed.message.message = [id; 16];
        signed.message.text = format!("local process {who} original");
        signed.signature = local
            .device_key
            .sign(&codec::message_digest(&signed.message).unwrap());
        let entry = outbox
            .enqueue(RequestId::from_bytes([id + 20; 16]), &signed)
            .unwrap();
        assert_eq!(entry.state, OutgoingState::Pending);
        assert_eq!(outbox.known_revision(), Ok(1));
    }
    // Both directions use different actual client PIDs and independently persisted
    // Noise identities. The supervisor never substitutes an in-process peer.
    for (sender, receiver, phase) in [
        ("alice", "bob", "first"),
        ("bob", "alice", "first"),
        ("alice", "bob", "reconnect"),
    ] {
        let address = root.join(format!("{receiver}-{phase}.ready"));
        let stop = root.join(format!("{receiver}-{phase}.stop"));
        let mut sidecar = Child::spawn(
            root, &evidence, "receiver", receiver, phase, &address, &stop,
        );
        sidecar.ready(&address, Duration::from_secs(8));
        let mut client = Child::spawn(root, &evidence, "client", sender, phase, &address, &stop);
        client.wait(Duration::from_secs(20));
        assert!(
            client
                .stdout()
                .contains("NF_CHAT_CLIENT_DELIVERED_HISTORY_ONE")
        );
        fs::write(&stop, b"STOP_OWNED_SYNTHETIC_SIDECAR\n").unwrap();
        sidecar.wait(Duration::from_secs(3));
        assert!(sidecar.stdout().contains("NF_CHAT_SIDECAR_STOPPED"));
        let snapshot = evidence.join(format!("{receiver}-first.closed.sqlite"));
        let receiver_bytes = fs::read(root.join(format!("receiver-{receiver}.sqlite"))).unwrap();
        if phase == "first" {
            fs::write(snapshot, &receiver_bytes).unwrap();
        } else {
            assert_eq!(
                fs::read(snapshot).unwrap(),
                receiver_bytes,
                "reconnect must preserve the original, aliases and frontier"
            );
        }
        let outbox_bytes = fs::read(root.join(format!("sender-{sender}.sqlite"))).unwrap();
        let snapshot = evidence.join(format!("{sender}-first.closed.outbox.sqlite"));
        if phase == "first" {
            fs::write(snapshot, &outbox_bytes).unwrap();
        } else {
            assert_eq!(
                fs::read(snapshot).unwrap(),
                outbox_bytes,
                "exact retry must not advance acknowledgement"
            );
        }
        assert!(
            whole.elapsed() < Duration::from_secs(120),
            "bounded two-client process flow"
        );
    }
    for (who, local, remote, id) in [
        ("alice", &fixture.alice, &fixture.bob, 81),
        ("bob", &fixture.bob, &fixture.alice, 82),
    ] {
        let known = KnownChatFrontiers {
            scope: fixture.policy.scope,
            revision: 1,
            membership_revision: 1,
        };
        let store = ChatStore::open_existing(
            root.join(format!("receiver-{who}.sqlite")),
            &fixture.policy,
            known,
        )
        .unwrap();
        assert_eq!(store.known_frontiers(), Ok(known));
        let profile = OutboxProfile::from_current(
            &fixture.policy,
            &fixture.state,
            author(local),
            author(remote),
        )
        .unwrap();
        let outbox =
            ClientOutbox::open_existing(root.join(format!("sender-{who}.sqlite")), &profile, 2)
                .unwrap();
        assert_eq!(outbox.known_revision(), Ok(2));
        assert!(matches!(
            outbox.entry([id; 16]).unwrap().unwrap().state,
            OutgoingState::Delivered(_)
        ));
    }
    println!(
        "NF_CHAT_LOCAL_PROCESS_FLOW clients=2 identities=2 sidecar_starts=3 client_starts=3 originals_per_receiver=1 exact_reconnect=1 remote_routes=0 elapsed_ms={}",
        whole.elapsed().as_millis()
    );
}
