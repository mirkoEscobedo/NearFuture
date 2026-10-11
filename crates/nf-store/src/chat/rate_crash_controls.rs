use super::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatPolicy, ChatRatePolicy, ChatStore,
    ChatStoreError, KnownChatFrontiers, ProofAttempt, ReceiverClock, SignedMessage, codec,
    legacy_schema,
};
use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    model::{DeviceProof, MembershipState, Scope},
    private_storage::{LocalIdentity, PrivateVault},
    signing::device_digest,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
struct Clock;
impl ReceiverClock for Clock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}
fn retain(root: &Path, name: &str, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
pub(super) fn cut(name: &str) {
    if std::env::var("NF_CHAT_RATE_ATOMIC_CUT").ok().as_deref() != Some(name) {
        return;
    }
    let root = PathBuf::from(
        std::env::var_os("NF_CHAT_RATE_ATOMIC_CHILD_EVIDENCE").expect("child evidence"),
    );
    assert!(root.is_absolute());
    retain(&root, "cut-reached.original.txt", name.as_bytes());
    // Process exit deliberately bypasses SQLite transaction/connection destructors.
    std::process::exit(73);
}
fn policy() -> ChatPolicy {
    ChatPolicy {
        scope: Scope {
            universe: UniverseId::from_bytes([1; 16]),
            history: HistoryId::from_bytes([2; 16]),
        },
    }
}
fn identity(root: &Path) -> LocalIdentity {
    PrivateVault::open(&root.join("vault"), &root.join("dummy-save-root"))
        .unwrap()
        .load_identity(b"atomic-chat-owner".to_vec())
        .unwrap()
}
fn signed(identity: &LocalIdentity) -> SignedMessage {
    let message = ChatMessage {
        scope: policy().scope,
        channel: Channel::General,
        author: Author {
            account: identity.public.account,
            device: identity.public.device,
        },
        message: [81; 16],
        sequence: 1,
        text: "atomic owner public original".into(),
    };
    let signature = identity
        .device_key
        .sign(&codec::message_digest(&message).unwrap());
    SignedMessage { message, signature }
}
fn post(
    store: &mut ChatStore,
    identity: &LocalIdentity,
    request: u8,
    signed: &SignedMessage,
) -> super::Result<super::ChatReceipt> {
    let request = RequestId::from_bytes([request; 16]);
    let issued = store.issue_challenge(
        ChallengeRequest::Post {
            request,
            message: signed,
        },
        &identity.public.peer,
    )?;
    let mut proof = DeviceProof {
        scope: policy().scope,
        account: identity.public.account,
        device: identity.public.device,
        frontier: issued.membership_revision,
        peer: identity.public.peer.clone(),
        challenge: issued.challenge,
        signature: [0; 64],
    };
    proof.signature = identity.device_key.sign(&device_digest(&proof).unwrap());
    store.post(
        request,
        signed,
        ProofAttempt {
            ticket: issued.ticket,
            proof: &proof,
            peer: &identity.public.peer,
        },
    )
}
#[test]
#[ignore = "owned child only; parent supplies disposable root and one exact cut"]
fn controlled_child() {
    let root = PathBuf::from(std::env::var_os("NF_CHAT_RATE_ATOMIC_ROOT").expect("owned root"));
    assert!(root.is_absolute());
    let who = identity(&root);
    let membership = MembershipState::bootstrap(policy().scope, &who.public).unwrap();
    let known = KnownChatFrontiers {
        scope: policy().scope,
        revision: 0,
        membership_revision: 0,
    };
    let operation = std::env::var("NF_CHAT_RATE_ATOMIC_OPERATION").unwrap();
    if operation == "migration" {
        let _store = ChatStore::migrate_v2_with_rate_policy(
            root.join("chat.sqlite"),
            &policy(),
            known,
            ChatRatePolicy::new(2, 2_000).unwrap(),
            Arc::new(Clock),
        )
        .unwrap();
    } else {
        assert!(operation == "original" || operation == "alias");
        let mut store = ChatStore::open_existing_with_clock(
            root.join("chat.sqlite"),
            &policy(),
            known,
            Arc::new(Clock),
        )
        .unwrap();
        assert_eq!(
            nf_identity::codec::encode_state(&store.current_membership().unwrap()).unwrap(),
            nf_identity::codec::encode_state(&membership).unwrap()
        );
        post(
            &mut store,
            &who,
            if operation == "alias" { 101 } else { 100 },
            &signed(&who),
        )
        .unwrap();
    }
    panic!("specified owner transaction cut was not reached");
}
struct OwnedChild(std::process::Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = self.0.kill();
        let until = Instant::now() + Duration::from_secs(3);
        while Instant::now() < until {
            if matches!(self.0.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // The managed parent Job remains the primary ownership cleanup boundary.
    }
}
fn child(root: &Path, operation: &str, stage: &str, evidence: &Path, deadline: Instant) {
    let stdout = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(evidence.join("child.stdout.original"))
        .unwrap();
    let stderr = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(evidence.join("child.stderr.original"))
        .unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "chat::rate_crash_controls::controlled_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NF_CHAT_RATE_ATOMIC_ROOT", root)
        .env("NF_CHAT_RATE_ATOMIC_OPERATION", operation)
        .env("NF_CHAT_RATE_ATOMIC_CUT", stage)
        .env("NF_CHAT_RATE_ATOMIC_CHILD_EVIDENCE", evidence)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = OwnedChild(command.spawn().unwrap());
    drop(command);
    let cutoff = std::cmp::min(deadline, Instant::now() + Duration::from_secs(10));
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= cutoff {
            let _ = child.0.kill();
            let reap = Instant::now() + Duration::from_secs(3);
            let mut reaped = false;
            while Instant::now() < reap {
                if child.0.try_wait().unwrap().is_some() {
                    reaped = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(reaped, "OWNED_CHILD_CLEANUP_INSUFFICIENT");
            panic!("OWNED_CHILD_TIMEOUT");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(
        status.code(),
        Some(73),
        "wrong child exit; inspect retained original streams"
    );
    assert_eq!(
        fs::read(evidence.join("cut-reached.original.txt")).unwrap(),
        stage.as_bytes()
    );
}
fn run_cut(operation: &str, stage: &str, committed: bool) {
    let until = Instant::now() + Duration::from_secs(20);
    assert!(
        std::env::var_os("NF_CHAT_RATE_ATOMIC_CUT").is_none(),
        "parent must not inherit a child-only cut selector"
    );
    let parent = PathBuf::from(
        std::env::var_os("NF_CHAT_RATE_ATOMIC_EVIDENCE_DIR").expect("external evidence"),
    );
    assert!(parent.is_absolute());
    let name = format!("{operation}-{stage}");
    let evidence = parent.join(&name);
    fs::create_dir(&evidence).unwrap();
    let scratch_parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp");
    let suffix: String = nf_identity::keys::random_id()
        .unwrap()
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    let scratch = scratch_parent.join(format!("chat-rate-atomic-{suffix}"));
    fs::create_dir(&scratch).unwrap();
    assert!(!evidence.starts_with(&scratch));
    retain(
        &evidence,
        "owned-scratch-path.original.txt",
        scratch.to_str().unwrap().as_bytes(),
    );
    // Only this test's disposable private identity lives here; original keys/saves are never opened.
    fs::create_dir(scratch.join("dummy-save-root")).unwrap();
    let vault =
        PrivateVault::create(&scratch.join("vault"), &scratch.join("dummy-save-root")).unwrap();
    let who = vault
        .create_identity(b"atomic-chat-owner".to_vec())
        .unwrap();
    let membership = MembershipState::bootstrap(policy().scope, &who.public).unwrap();
    let database = scratch.join("chat.sqlite");
    let rate = ChatRatePolicy::new(2, 2_000).unwrap();
    let message = signed(&who);
    if operation == "migration" {
        crate::schema::reserve(&database).unwrap();
        let mut connection = crate::schema::connection(&database).unwrap();
        crate::schema::configure(&connection).unwrap();
        legacy_schema::initialize_test_v2(&mut connection, &policy(), &membership).unwrap();
        drop(connection);
    } else {
        let mut store = ChatStore::create_with_rate_policy_and_clock(
            &database,
            &policy(),
            &membership,
            rate,
            Arc::new(Clock),
        )
        .unwrap();
        if operation == "alias" {
            post(&mut store, &who, 100, &message).unwrap();
        }
        drop(store);
    }
    retain(&evidence, "before.sqlite", &fs::read(&database).unwrap());
    retain(
        &evidence,
        "signed.original.bin",
        &codec::encode_signed_message(&message).unwrap(),
    );
    child(&scratch, operation, stage, &evidence, until);
    // Opening the owned disposable copy performs real SQLite hot-journal recovery, never original evidence.
    let known = KnownChatFrontiers {
        scope: policy().scope,
        revision: 0,
        membership_revision: 0,
    };
    if operation == "migration" {
        if !committed {
            assert!(matches!(
                ChatStore::open_existing_with_clock(&database, &policy(), known, Arc::new(Clock)),
                Err(ChatStoreError::UnsupportedProfile)
            ));
            let connection = crate::schema::connection(&database).unwrap();
            crate::schema::configure(&connection).unwrap();
            assert_eq!(
                legacy_schema::verify(&connection, &policy())
                    .unwrap()
                    .revision,
                0
            );
            drop(connection);
        }
        let store = if committed {
            ChatStore::open_existing_with_clock(&database, &policy(), known, Arc::new(Clock))
                .unwrap()
        } else {
            ChatStore::migrate_v2_with_rate_policy(
                &database,
                &policy(),
                known,
                rate,
                Arc::new(Clock),
            )
            .unwrap()
        };
        assert_eq!(store.known_frontiers().unwrap().revision, 0);
        let (_, _, tokens, fraction) = store.atomic_test_snapshot(who.public.account).unwrap();
        assert_eq!(tokens, 0);
        assert_eq!(fraction, 0);
        drop(store);
    } else {
        let mut store =
            ChatStore::open_existing_with_clock(&database, &policy(), known, Arc::new(Clock))
                .unwrap();
        let (requests, messages, tokens, fraction) =
            store.atomic_test_snapshot(who.public.account).unwrap();
        let old_count = if operation == "alias" { 1 } else { 0 };
        assert_eq!(requests, old_count + usize::from(committed));
        assert_eq!(messages, usize::from(operation == "alias" || committed));
        assert_eq!(tokens, 2 - (old_count as u16) - u16::from(committed));
        assert_eq!(fraction, 0);
        let before_retry = fs::read(&database).unwrap();
        let request = if operation == "alias" { 101 } else { 100 };
        let _receipt = post(&mut store, &who, request, &message).unwrap();
        let after_retry = fs::read(&database).unwrap();
        if committed {
            assert_eq!(
                before_retry, after_retry,
                "recorded accepted request must remain free"
            );
        }
        let (_, _, tokens, fraction) = store.atomic_test_snapshot(who.public.account).unwrap();
        assert_eq!(tokens, 2 - (old_count as u16) - 1);
        assert_eq!(fraction, 0);
        assert_eq!(store.known_frontiers().unwrap().revision, 1);
        drop(store);
    }
    retain(
        &evidence,
        "after-recovery-and-retry.sqlite",
        &fs::read(&database).unwrap(),
    );
    // Retain this owned scratch directory on success/failure for Root custody; never archive private key bytes as evidence.
    assert!(Instant::now() < until);
}
macro_rules! cut_test {
    ($name:ident,$operation:literal,$stage:literal,$committed:literal) => {
        #[test]
        fn $name() {
            run_cut($operation, $stage, $committed);
        }
    };
}
cut_test!(
    original_before_debit,
    "original",
    "post-before-debit",
    false
);
cut_test!(original_after_debit, "original", "post-after-debit", false);
cut_test!(
    original_after_message,
    "original",
    "post-after-original",
    false
);
cut_test!(
    original_after_request,
    "original",
    "post-after-request",
    false
);
cut_test!(
    original_before_commit,
    "original",
    "post-before-commit",
    false
);
cut_test!(original_after_commit, "original", "post-after-commit", true);
cut_test!(alias_after_debit, "alias", "post-after-debit", false);
cut_test!(alias_after_request, "alias", "post-after-request", false);
cut_test!(alias_after_commit, "alias", "post-after-commit", true);
cut_test!(
    migration_after_ddl,
    "migration",
    "migration-after-ddl",
    false
);
cut_test!(
    migration_after_rows,
    "migration",
    "migration-after-rows",
    false
);
cut_test!(
    migration_after_version,
    "migration",
    "migration-after-version",
    false
);
cut_test!(
    migration_after_commit,
    "migration",
    "migration-after-commit",
    true
);
