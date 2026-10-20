use super::{ManagedError, bind_path};
use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let path = root.join(format!(
            "managed-path-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .canonicalize()
            .unwrap();
        let path = self.0.canonicalize().unwrap();
        assert!(path.starts_with(&root) && path != root);
        let _ = std::fs::remove_dir_all(path);
    }
}
fn database(scratch: &Scratch) -> (Connection, PathBuf) {
    let path = scratch.0.join("source.sqlite");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE main.history_meta(value INTEGER)")
        .unwrap();
    (connection, path.canonicalize().unwrap())
}

#[test]
fn exact_main_path_without_temp_is_accepted() {
    let scratch = Scratch::new();
    let (connection, path) = database(&scratch);
    assert_eq!(bind_path(&connection, &path), Ok(()));
}

#[test]
fn integrity_check_and_an_empty_temp_schema_are_harmless() {
    let scratch = Scratch::new();
    let (connection, path) = database(&scratch);
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(result, "ok");
    connection
        .execute_batch("CREATE TEMP TABLE scratch(value); DROP TABLE temp.scratch")
        .unwrap();
    assert_eq!(bind_path(&connection, &path), Ok(()));
}

#[test]
fn a_different_existing_main_path_is_refused() {
    let scratch = Scratch::new();
    let (connection, path) = database(&scratch);
    let other = scratch.0.join("other.sqlite");
    std::fs::copy(&path, &other).unwrap();
    assert_eq!(
        bind_path(&connection, &other.canonicalize().unwrap()),
        Err(ManagedError::SourceMismatch)
    );
}

#[test]
fn an_unexpected_attached_database_is_refused_even_without_a_file() {
    let scratch = Scratch::new();
    let (connection, path) = database(&scratch);
    connection
        .execute_batch("ATTACH DATABASE ':memory:' AS extra")
        .unwrap();
    assert_eq!(
        bind_path(&connection, &path),
        Err(ManagedError::SourceMismatch)
    );
}

#[test]
fn temp_objects_cannot_shadow_or_modify_main_tables() {
    for ddl in [
        "CREATE TEMP TABLE history_meta(value INTEGER)",
        "CREATE TEMP VIEW history_meta AS SELECT 1 AS value",
        "CREATE TEMP TRIGGER watcher AFTER INSERT ON main.history_meta BEGIN SELECT 1; END",
    ] {
        let scratch = Scratch::new();
        let (connection, path) = database(&scratch);
        connection.execute_batch(ddl).unwrap();
        assert_eq!(
            bind_path(&connection, &path),
            Err(ManagedError::SourceMismatch)
        );
    }
}

#[test]
fn a_memory_main_cannot_masquerade_as_an_existing_file() {
    let scratch = Scratch::new();
    let (_file, path) = database(&scratch);
    let memory = Connection::open_in_memory().unwrap();
    assert_eq!(bind_path(&memory, &path), Err(ManagedError::SourceMismatch));
}
