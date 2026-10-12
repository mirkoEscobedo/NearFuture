use super::{
    ChatPolicy, ChatRatePolicy, ChatReceipt, ChatStore, ChatStoreError, KnownChatFrontiers,
    ReceiverClock, codec, legacy_schema, rate_sql, schema,
};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

struct Clock(Duration);
impl ReceiverClock for Clock {
    fn now(&self) -> Duration {
        self.0
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Original {
    id: [u8; 16],
    signed: Vec<u8>,
    receipt: ChatReceipt,
}
#[derive(Debug, PartialEq, Eq)]
struct Request {
    id: Vec<u8>,
    binding: [u8; 32],
    message: [u8; 16],
}
#[derive(Debug, PartialEq, Eq)]
struct Durable {
    revision: u64,
    originals: Vec<Original>,
    requests: Vec<Request>,
    membership: Vec<u8>,
    policy: Vec<u8>,
    policy_digest: Vec<u8>,
}
fn durable(connection: &rusqlite::Connection, policy: &ChatPolicy, v2: bool) -> Durable {
    let state = if v2 {
        legacy_schema::verify(connection, policy)
    } else {
        schema::verify(connection, policy)
    }
    .expect("NONEMPTY_V2_FIXTURE_OR_MIGRATED_LEDGER_VERIFICATION_FAILED");
    let members = if v2 {
        legacy_schema::current(connection, policy.scope)
    } else {
        schema::current(connection, policy.scope)
    }
    .unwrap();
    Durable {
        revision: state.revision,
        originals: state
            .messages
            .iter()
            .map(|(id, entry)| Original {
                id: *id,
                signed: codec::encode_signed_message(&entry.signed).unwrap(),
                receipt: entry.receipt,
            })
            .collect(),
        requests: state
            .requests
            .iter()
            .map(|(id, entry)| Request {
                id: id.as_bytes().to_vec(),
                binding: entry.binding,
                message: entry.message,
            })
            .collect(),
        membership: nf_identity::codec::encode_state(&members).unwrap(),
        policy: codec::policy_bytes(policy).to_vec(),
        policy_digest: codec::policy_digest(policy).to_vec(),
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
    assert_eq!(fs::read(root.join(name)).unwrap(), bytes);
}
fn input(root: &Path, name: &str, exact_bytes: u64) -> Vec<u8> {
    let path = root.join(name);
    let metadata = fs::symlink_metadata(&path).expect("OWNED_FIXTURE_INPUT_MISSING");
    assert!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "OWNED_FIXTURE_INPUT_TYPE_REFUSED"
    );
    assert_eq!(
        metadata.len(),
        exact_bytes,
        "OWNED_FIXTURE_INPUT_SIZE_REFUSED"
    );
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len() as u64, exact_bytes);
    bytes
}
fn remaining(until: Instant, stage: &str) {
    assert!(
        Instant::now() < until,
        "SUPPLEMENTAL_TIME_BUDGET_EXHAUSTED_AT_{stage}"
    );
}
#[test]
fn genuine_nonempty_v2_owned_copy_preserves_history_requests_and_empty_migration_balance() {
    let started = Instant::now();
    let until = started + Duration::from_secs(20);
    assert!(
        std::env::var_os("NF_CHAT_RATE_ATOMIC_CUT").is_none(),
        "CHILD_ONLY_CUT_SELECTOR_REFUSED"
    );
    let fixture = super::rate_test_inputs::nonempty_fixture();
    let generated_synthetic = fixture.generated_synthetic;
    let inputs = fixture.inputs;
    let parent = fixture.evidence;
    assert!(inputs.is_absolute() && parent.is_absolute());
    let inputs = inputs.canonicalize().unwrap();
    let parent = parent.canonicalize().unwrap();
    assert!(
        !inputs.starts_with(&parent) && !parent.starts_with(&inputs),
        "OWNED_INPUT_AND_OUTPUT_BOUNDARIES_OVERLAP"
    );
    // Supplied mode remains Root hash-qualified copies; absent inputs use separately tagged
    // fresh synthetic signed v2 data, never retained historical fixtures or signer keys.
    let original_bytes = input(&inputs, "genuine-v2.sqlite.bin", 45_056);
    let signed_bytes = input(&inputs, "signed.original.bin", 193);
    let signed = codec::decode_signed_message(&signed_bytes).unwrap();
    assert_eq!(codec::encode_signed_message(&signed).unwrap(), signed_bytes);
    let policy = ChatPolicy {
        scope: signed.message.scope,
    };
    let evidence = parent.join(if generated_synthetic {
        "generated-signed-nonempty-v2"
    } else {
        "genuine-nonempty-v2"
    });
    fs::create_dir(&evidence).expect("FRESH_OWNED_EVIDENCE_LEAF_REQUIRED");
    if generated_synthetic {
        retain(
            &evidence,
            "fixture-origin.original.txt",
            b"GENERATED_SYNTHETIC_SIGNED_V2_NOT_HISTORICAL_RETAINED_FIXTURE\n",
        );
    }
    retain(&evidence, "before.sqlite", &original_bytes);
    retain(&evidence, "signed.original.bin", &signed_bytes);
    retain(&evidence, "owned.sqlite", &original_bytes);
    let database = evidence.join("owned.sqlite");
    remaining(until, "OWNED_COPY_CREATED");
    let connection = crate::schema::connection(&database).unwrap();
    crate::schema::configure(&connection).unwrap();
    let state = legacy_schema::verify(&connection, &policy)
        .expect("NONEMPTY_V2_FIXTURE_INSUFFICIENT_EVIDENCE");
    assert!(
        state.revision > 0
            && !state.messages.is_empty()
            && state.requests.len() > state.messages.len(),
        "NONEMPTY_V2_ORIGINAL_AND_ALIAS_EVIDENCE_REQUIRED"
    );
    assert_eq!(
        state
            .messages
            .get(&signed.message.message)
            .expect("CAPTURED_SIGNED_ORIGINAL_NOT_IN_LEDGER")
            .signed,
        signed
    );
    if generated_synthetic {
        assert_eq!(state.revision, 1, "SYNTHETIC_V2_EXACT_ORIGINAL_CURSOR");
        assert_eq!(state.messages.len(), 1, "SYNTHETIC_V2_ONE_SIGNED_ORIGINAL");
        assert_eq!(state.requests.len(), 10, "SYNTHETIC_V2_NINE_FRESH_ALIASES");
    }
    let members = legacy_schema::current(&connection, policy.scope).unwrap();
    assert!(!members.accounts.is_empty());
    let known = KnownChatFrontiers {
        scope: policy.scope,
        revision: state.revision,
        membership_revision: members.revision,
    };
    let before = durable(&connection, &policy, true);
    retain(
        &evidence,
        "before-canonical.original.txt",
        format!("{before:?}").as_bytes(),
    );
    drop(state);
    drop(connection);
    remaining(until, "VERIFIED_V2_BEFORE_OPEN");
    assert!(matches!(
        ChatStore::open_existing_with_clock(
            &database,
            &policy,
            known,
            Arc::new(Clock(Duration::ZERO))
        ),
        Err(ChatStoreError::UnsupportedProfile)
    ));
    let rate = ChatRatePolicy::default();
    remaining(until, "BEFORE_EXPLICIT_MIGRATION");
    let store = ChatStore::migrate_v2_with_rate_policy(
        &database,
        &policy,
        known,
        rate,
        Arc::new(Clock(Duration::ZERO)),
    )
    .unwrap();
    assert_eq!(store.receiver_rate_policy(), Ok((rate, 0)));
    let frontiers = store.known_frontiers().unwrap();
    assert_eq!(frontiers.scope, known.scope);
    assert_eq!(frontiers.revision, known.revision);
    assert_eq!(frontiers.membership_revision, known.membership_revision);
    assert_eq!(
        nf_identity::codec::encode_state(&store.current_membership().unwrap()).unwrap(),
        before.membership
    );
    for account in members.accounts.keys() {
        assert_eq!(
            store.atomic_test_snapshot(*account).unwrap(),
            (before.requests.len(), before.originals.len(), 0, 0)
        );
    }
    drop(store);
    remaining(until, "AFTER_EXPLICIT_MIGRATION");
    let connection = crate::schema::connection(&database).unwrap();
    crate::schema::configure(&connection).unwrap();
    let after = durable(&connection, &policy, false);
    assert_eq!(
        after, before,
        "MIGRATION_CHANGED_DURABLE_HISTORY_OR_REQUEST_BINDING"
    );
    let snapshot = rate_sql::verify_all(&connection, &members).unwrap();
    assert_eq!(snapshot.generation, 0);
    assert_eq!(snapshot.policy, rate);
    assert_eq!(snapshot.accounts.len(), members.accounts.len());
    for balance in snapshot.accounts.values() {
        assert_eq!((balance.tokens, balance.fractional_ns), (0, 0));
    }
    retain(
        &evidence,
        "after-canonical.original.txt",
        format!("{after:?}").as_bytes(),
    );
    drop(connection);
    let after_migration = fs::read(&database).unwrap();
    retain(&evidence, "after-migration.sqlite", &after_migration);
    remaining(until, "BEFORE_FAR_FUTURE_REOPEN");
    let store = ChatStore::open_existing_with_clock(
        &database,
        &policy,
        known,
        Arc::new(Clock(Duration::from_secs(100_000))),
    )
    .unwrap();
    assert_eq!(store.receiver_rate_policy(), Ok((rate, 0)));
    for account in members.accounts.keys() {
        assert_eq!(
            store.atomic_test_snapshot(*account).unwrap(),
            (before.requests.len(), before.originals.len(), 0, 0)
        );
    }
    drop(store);
    assert_eq!(
        fs::read(&database).unwrap(),
        after_migration,
        "REOPEN_WITHOUT_POST_CHANGED_DURABLE_DATABASE"
    );
    retain(
        &evidence,
        "after-reopen.sqlite",
        &fs::read(&database).unwrap(),
    );
    assert_eq!(
        input(&inputs, "genuine-v2.sqlite.bin", 45_056),
        original_bytes
    );
    assert_eq!(input(&inputs, "signed.original.bin", 193), signed_bytes);
    remaining(until, "FINAL_PRESERVATION");
    retain(
        &evidence,
        "elapsed-ns.original.txt",
        started.elapsed().as_nanos().to_string().as_bytes(),
    );
    // No signer keys are retained: this test makes no postmigration authenticated retry claim.
}
