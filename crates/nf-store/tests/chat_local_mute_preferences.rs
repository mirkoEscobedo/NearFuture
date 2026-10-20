mod chat_support;
use chat_support::Fixture;
use nf_store::chat::{
    ChatStore, ChatStoreError, HistoryEntry, HistoryPage,
    local_preferences::{load_local_mutes, save_new_local_mutes},
    local_view::project_local_mutes,
};
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
fn authenticated_history_local_mute_preferences_survive_close_reopen_without_store_writes() {
    let start = Instant::now();
    let evidence = PathBuf::from(
        std::env::var_os("NF_CHAT_LOCAL_MUTE_PREFERENCES_EVIDENCE_DIR")
            .expect("Root must supply one fresh owned external evidence directory"),
    );
    assert!(evidence.is_absolute());
    let evidence = evidence.canonicalize().unwrap();
    assert!(evidence.is_dir());
    let fixture = Fixture::new();
    assert!(!evidence.starts_with(fixture.scratch.0.canonicalize().unwrap()));
    let database = fixture.database();
    let mut store = ChatStore::create(&database, &fixture.policy, &fixture.membership).unwrap();
    let signed = fixture.signed_message();
    assert_eq!(
        fixture
            .post(&mut store, 121, &signed)
            .unwrap()
            .receiver_cursor,
        1
    );
    let source = fixture.permitted_history(&mut store, 122).unwrap();
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
    let preferences = evidence.join("local-mute.preferences.original.bin");
    assert!(!preferences.exists());
    let viewer = fixture.bob.public.account;
    let muted = [fixture.alice.public.account];
    // Intended genuine RED after authenticated source/captures: UnsupportedOperation.
    assert_eq!(
        save_new_local_mutes(&preferences, fixture.policy.scope, viewer, &muted),
        Ok(()),
    );
    let mut expected = b"NF-MUTE1".to_vec();
    expected.extend_from_slice(fixture.policy.scope.universe.as_bytes());
    expected.extend_from_slice(fixture.policy.scope.history.as_bytes());
    expected.extend_from_slice(viewer.as_bytes());
    expected.push(1);
    expected.extend_from_slice(muted[0].as_bytes());
    assert_eq!(expected.len(), 73);
    assert_eq!(fs::read(&preferences).unwrap(), expected);
    let restored = load_local_mutes(&preferences, fixture.policy.scope, viewer).unwrap();
    assert_eq!(restored, muted);
    assert_eq!(
        project_local_mutes(fixture.policy.scope, &restored, &source),
        Ok(HistoryPage {
            entries: Vec::new(),
            next_cursor: 1,
        }),
    );
    assert_eq!(source, literal);
    assert_eq!(
        save_new_local_mutes(&preferences, fixture.policy.scope, viewer, &[]),
        Err(ChatStoreError::AlreadyExists),
    );
    assert_eq!(fs::read(&preferences).unwrap(), expected);
    assert_eq!(fs::read(&database).unwrap(), before);
    let mut reopened = ChatStore::open_existing(&database, &fixture.policy, known).unwrap();
    assert_eq!(reopened.known_frontiers().unwrap(), known);
    let reloaded_source = fixture.permitted_history(&mut reopened, 123).unwrap();
    assert_eq!(reloaded_source, literal);
    assert_eq!(reopened.known_frontiers().unwrap(), known);
    drop(reopened);
    let reloaded_preferences =
        load_local_mutes(&preferences, fixture.policy.scope, viewer).unwrap();
    assert_eq!(reloaded_preferences, muted);
    assert_eq!(
        project_local_mutes(
            fixture.policy.scope,
            &reloaded_preferences,
            &reloaded_source
        ),
        Ok(HistoryPage {
            entries: Vec::new(),
            next_cursor: 1,
        }),
    );
    let after = fs::read(&database).unwrap();
    retain(
        &evidence,
        "authenticated-history.after.original.sqlite",
        &after,
    );
    retain(
        &evidence,
        "local-mute.preferences.after-reopen.original.bin",
        &fs::read(&preferences).unwrap(),
    );
    assert_eq!(after, before);
    assert!(start.elapsed() < Duration::from_secs(20));
}
