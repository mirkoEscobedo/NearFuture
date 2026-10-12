//! Owned test inputs only. Generated fixtures never represent retained historical evidence.
use super::{Author, Channel, ChatMessage, ChatPolicy, SignedMessage, codec, legacy_schema, write};
use nf_contract::identity::{HistoryId, RequestId, UniverseId};
use nf_identity::{
    keys,
    model::{MembershipState, Scope},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn ordinary_root(label: &str) -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let parent = workspace.join(".tmp");
    match fs::create_dir(&parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => panic!("OWNED_TEST_PARENT_CREATE_FAILED: {error}"),
    }
    let metadata = fs::symlink_metadata(&parent).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            metadata.file_attributes() & 0x400,
            0,
            "OWNED_TEST_PARENT_REPARSE_REFUSED"
        );
    }
    let suffix: String = keys::random_id()
        .unwrap()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let root = parent.join(format!("{label}-{suffix}"));
    fs::create_dir(&root).expect("FRESH_OWNED_TEST_ROOT_REQUIRED");
    root.canonicalize().unwrap()
}

pub(super) fn atomic_evidence_parent() -> PathBuf {
    if let Some(root) = std::env::var_os("NF_CHAT_RATE_ATOMIC_EVIDENCE_DIR") {
        return PathBuf::from(root);
    }
    ordinary_root("chat-rate-atomic-evidence")
}

pub(super) struct NonemptyFixture {
    pub inputs: PathBuf,
    pub evidence: PathBuf,
    pub generated_synthetic: bool,
}

fn retain(root: &Path, name: &str, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
    assert_eq!(fs::read(root.join(name)).unwrap(), bytes);
}

pub(super) fn nonempty_fixture() -> NonemptyFixture {
    match (
        std::env::var_os("NF_CHAT_NONEMPTY_V2_OWNED_INPUT_DIR"),
        std::env::var_os("NF_CHAT_NONEMPTY_V2_EVIDENCE_DIR"),
    ) {
        (Some(inputs), Some(evidence)) => NonemptyFixture {
            inputs: PathBuf::from(inputs),
            evidence: PathBuf::from(evidence),
            generated_synthetic: false,
        },
        (None, None) => generated_nonempty_fixture(),
        _ => panic!("PARTIAL_EXTERNAL_NONEMPTY_V2_FIXTURE_CONFIGURATION_REFUSED"),
    }
}

fn generated_nonempty_fixture() -> NonemptyFixture {
    let inputs = ordinary_root("chat-nonempty-v2-generated-inputs");
    let evidence = ordinary_root("chat-nonempty-v2-generated-evidence");
    let policy = ChatPolicy {
        scope: Scope {
            universe: UniverseId::from_bytes([41; 16]),
            history: HistoryId::from_bytes([42; 16]),
        },
    };
    // Both secrets are fresh RAM-only values; no vault, retained key or original store is opened.
    let (public, _account_key, device_key) =
        keys::generate_identity(b"generated-synthetic-v2".to_vec()).unwrap();
    let membership = MembershipState::bootstrap(policy.scope, &public).unwrap();
    let message = ChatMessage {
        scope: policy.scope,
        channel: Channel::General,
        author: Author {
            account: public.account,
            device: public.device,
        },
        message: [43; 16],
        sequence: 1,
        text: "synthetic v2 message".into(),
    };
    let signed = SignedMessage {
        signature: device_key.sign(&codec::message_digest(&message).unwrap()),
        message,
    };
    codec::verify_message(&signed, &membership).unwrap();
    let signed_bytes = codec::encode_signed_message(&signed).unwrap();
    assert_eq!(
        signed_bytes.len(),
        193,
        "FIXED_SYNTHETIC_SIGNED_MESSAGE_SHAPE"
    );
    let database = inputs.join("fixture-build.sqlite");
    crate::schema::reserve(&database).unwrap();
    let mut connection = crate::schema::connection(&database).unwrap();
    crate::schema::configure(&connection).unwrap();
    legacy_schema::initialize_test_v2(&mut connection, &policy, &membership).unwrap();
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .unwrap();
    // Populate only a test-owned v2 fixture. This is not a receiver proof/admission qualification.
    for number in 100u8..110 {
        let request = RequestId::from_bytes([number; 16]);
        let state = legacy_schema::verify(&tx, &policy).unwrap();
        let prepared = write::prepare(&policy, &state, request, &signed).unwrap();
        write::apply(&tx, &state, request, &signed, &prepared).unwrap();
    }
    tx.commit().unwrap();
    let state = legacy_schema::verify(&connection, &policy).unwrap();
    assert_eq!(state.revision, 1);
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.requests.len(), 10);
    assert_eq!(state.messages.get(&[43; 16]).unwrap().signed, signed);
    drop(state);
    drop(connection);
    let original_bytes = fs::read(&database).unwrap();
    assert_eq!(
        original_bytes.len(),
        45_056,
        "FIXED_SYNTHETIC_V2_DATABASE_SHAPE"
    );
    retain(&inputs, "genuine-v2.sqlite.bin", &original_bytes);
    retain(&inputs, "signed.original.bin", &signed_bytes);
    retain(
        &inputs,
        "fixture-origin.original.txt",
        b"GENERATED_SYNTHETIC_SIGNED_V2_NOT_HISTORICAL_RETAINED_FIXTURE\none original; ten requests; nine aliases; RAM-only fresh keys\n",
    );
    NonemptyFixture {
        inputs,
        evidence,
        generated_synthetic: true,
    }
}
