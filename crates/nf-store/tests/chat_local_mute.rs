mod chat_support;
use chat_support::Fixture;
use nf_store::chat::{ChatStore, HistoryEntry, HistoryPage, local_view::project_local_mutes};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn retain(directory: &Path, name: &str, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

#[test]
fn authenticated_source_history_can_be_muted_locally_without_deletion_or_cursor_rewind() {
    let start = Instant::now();
    let evidence = PathBuf::from(
        std::env::var_os("NF_CHAT_LOCAL_MUTE_EVIDENCE_DIR")
            .expect("Root must provide one owned external evidence directory"),
    );
    assert!(evidence.is_absolute());
    let evidence = evidence.canonicalize().unwrap();
    assert!(evidence.is_dir());
    let fixture = Fixture::new();
    assert!(!evidence.starts_with(fixture.scratch.0.canonicalize().unwrap()));
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let signed = fixture.signed_message();
    let receipt = fixture.post(&mut store, 101, &signed).unwrap();
    assert_eq!(receipt.receiver_cursor, 1);
    let source = fixture.permitted_history(&mut store, 111).unwrap();
    let literal = HistoryPage {
        entries: vec![HistoryEntry {
            receiver_cursor: 1,
            signed: signed.clone(),
        }],
        next_cursor: 1,
    };
    assert_eq!(source, literal);
    let known = store.known_frontiers().unwrap();
    drop(store);
    let before = fs::read(&database).unwrap();
    retain(
        &evidence,
        "authenticated-history.before.original.sqlite",
        &before,
    );
    retain(
        &evidence,
        "signed-original.before.original.bin",
        &nf_store::chat::codec::encode_signed_message(&signed).unwrap(),
    );
    // Intended genuine RED: already authenticated source versus UnsupportedOperation.
    let muted = project_local_mutes(
        fixture.policy.scope,
        &[fixture.alice.public.account],
        &source,
    );
    assert_eq!(
        muted,
        Ok(HistoryPage {
            entries: Vec::new(),
            next_cursor: 1,
        }),
    );
    assert_eq!(source, literal);
    assert_eq!(
        project_local_mutes(fixture.policy.scope, &[], &source),
        Ok(literal.clone())
    );
    assert_eq!(fs::read(&database).unwrap(), before);
    let mut reopened = ChatStore::open_existing(&database, &fixture.policy, known).unwrap();
    assert_eq!(reopened.known_frontiers().unwrap(), known);
    assert_eq!(
        fixture.permitted_history(&mut reopened, 112).unwrap(),
        literal
    );
    assert_eq!(reopened.known_frontiers().unwrap(), known);
    drop(reopened);
    let after = fs::read(&database).unwrap();
    retain(
        &evidence,
        "authenticated-history.after.original.sqlite",
        &after,
    );
    assert_eq!(after, before);
    assert!(start.elapsed() < Duration::from_secs(20));
}
