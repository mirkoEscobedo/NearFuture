//! Invalid archives and occupied outputs cannot gain an isolated copy by side effect.
mod miniature_restore_refusal_support;
mod miniature_support;
use miniature_restore_refusal_support::{RetainedBackup, tree};
use nf_store::{
    StoreError,
    miniature::{IsolatedRestoreConfirmation, MiniatureReader, MiniatureStoreError},
};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};

fn open_fixture(path: &Path) -> Connection {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    connection.busy_timeout(Duration::from_millis(250)).unwrap();
    connection
}

fn state_digest(connection: &Connection) -> (Vec<u8>, Vec<u8>) {
    connection
        .query_row(
            "SELECT state,digest FROM history_meta WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

fn assert_refusal(
    fixture: &RetainedBackup,
    backup: &Path,
    target: &Path,
    expected: StoreError,
) {
    let before = tree(&fixture.root);
    let occupied = target.exists();
    assert_eq!(
        MiniatureReader::restore_backup_to_new_path(
            backup,
            target,
            fixture.known,
            IsolatedRestoreConfirmation::NewReadOnlyCopy,
        )
        .err(),
        Some(MiniatureStoreError::Storage(expected))
    );
    assert_eq!(target.exists(), occupied);
    assert_eq!(tree(&fixture.root), before);
    fixture.assert_original();
    assert_eq!(tree(&fixture.root), before);
}

#[test]
fn corrupt_history_and_wrong_profile_are_refused_before_any_output_effect() {
    let fixture = RetainedBackup::new("source-admission");
    let corrupt = fixture.root.join("missing-claim-record.sqlite");
    fs::copy(&fixture.backup, &corrupt).unwrap();
    let connection = open_fixture(&corrupt);
    let complete_state = state_digest(&connection);
    assert_eq!(complete_state.1, Sha256::digest(&complete_state.0).to_vec());
    // Remove an actual durable claim while leaving valid State bytes/hash intact.
    assert_eq!(connection.execute("DELETE FROM journal", []).unwrap(), 1);
    assert_eq!(state_digest(&connection), complete_state);
    let integrity: String = connection
        .query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    connection.close().map_err(|(_, error)| error).unwrap();

    let wrong_profile = fixture.root.join("wrong-profile.sqlite");
    fs::copy(&fixture.backup, &wrong_profile).unwrap();
    let connection = open_fixture(&wrong_profile);
    connection.pragma_update(None, "user_version", 1).unwrap();
    assert_eq!(
        connection
            .pragma_query_value::<i32, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        1
    );
    connection.close().map_err(|(_, error)| error).unwrap();

    let corrupt_target = fixture.root.join("corrupt-target.sqlite");
    let profile_target = fixture.root.join("profile-target.sqlite");
    assert!(!corrupt_target.exists() && !profile_target.exists());
    assert_refusal(&fixture, &corrupt, &corrupt_target, StoreError::Corrupt);
    assert_refusal(
        &fixture,
        &wrong_profile,
        &profile_target,
        StoreError::UnsupportedSchema,
    );
}

#[test]
fn occupied_outputs_source_aliases_and_companions_are_preserved_without_creation() {
    let fixture = RetainedBackup::new("destination-admission");
    let occupied = fixture.root.join("occupied.sqlite");
    fs::write(&occupied, b"existing caller-owned destination").unwrap();
    let alias = fixture.root.join("original-hardlink.sqlite");
    fs::hard_link(&fixture.source, &alias).unwrap();
    for target in [&occupied, &fixture.backup, &fixture.source, &alias] {
        assert_refusal(&fixture, &fixture.backup, target, StoreError::AlreadyExists);
    }

    for (name, suffix) in [("wal", "-wal"), ("shm", "-shm"), ("journal", "-journal")] {
        let target = fixture.root.join(format!("occupied-{name}.sqlite"));
        let mut companion = target.as_os_str().to_os_string();
        companion.push(suffix);
        let companion = Path::new(&companion);
        fs::write(companion, b"existing caller-owned SQLite companion").unwrap();
        assert!(!target.exists());
        assert_refusal(&fixture, &fixture.backup, &target, StoreError::AlreadyExists);
        assert!(!target.exists());
    }
}
