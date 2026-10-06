use crate::error::{Result, StoreError};
use rusqlite::{Connection, OpenFlags, limits::Limit};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, path::Path, time::Duration};
pub(crate) const APPLICATION_ID: i32 = 0x4e46_5354;
pub(crate) const SCHEMA_VERSION: i32 = 1;
pub(crate) const SCHEMA: &str = r#"
CREATE TABLE history_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), state BLOB NOT NULL CHECK(length(state)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32)) STRICT;
CREATE TABLE snapshots (revision BLOB PRIMARY KEY CHECK(length(revision)=8), state BLOB NOT NULL CHECK(length(state)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32)) STRICT;
CREATE TABLE journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 3), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=1048576), state BLOB NOT NULL CHECK(length(state)<=1048576)) STRICT;
CREATE TABLE events (sequence BLOB PRIMARY KEY CHECK(length(sequence)=8), body BLOB NOT NULL CHECK(length(body)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE aggregate_state (aggregate_id BLOB PRIMARY KEY CHECK(length(aggregate_id)=16), revision BLOB NOT NULL CHECK(length(revision)=8)) STRICT;
CREATE TABLE module_state (provider_id BLOB PRIMARY KEY CHECK(length(provider_id)=16), draws BLOB NOT NULL CHECK(length(draws)=8), cooldown BLOB NOT NULL CHECK(length(cooldown)=8)) STRICT;
CREATE TABLE request_outcomes (request_id BLOB PRIMARY KEY CHECK(length(request_id)=16), binding_digest BLOB NOT NULL CHECK(length(binding_digest)=32), intent_digest BLOB NOT NULL CHECK(length(intent_digest)=32), operation_id BLOB NOT NULL CHECK(length(operation_id)=16), phase INTEGER NOT NULL CHECK(phase IN (1,2)), event_seq BLOB CHECK(event_seq IS NULL OR length(event_seq)=8), rejection INTEGER CHECK(rejection IS NULL OR rejection BETWEEN 0 AND 17)) STRICT;
CREATE TABLE reservations (operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16), market_id BLOB NOT NULL CHECK(length(market_id)=16), amount BLOB NOT NULL CHECK(length(amount)=8)) STRICT;
CREATE TABLE outbox (operation_id BLOB PRIMARY KEY CHECK(length(operation_id)=16), sequence BLOB NOT NULL CHECK(length(sequence)=8), batch_digest BLOB NOT NULL CHECK(length(batch_digest)=32), rejection INTEGER NOT NULL CHECK(rejection BETWEEN 0 AND 17)) STRICT;
CREATE TABLE membership (universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), public_state BLOB NOT NULL CHECK(length(public_state)<=262144), digest BLOB NOT NULL CHECK(length(digest)=32), PRIMARY KEY(universe,history)) STRICT;
"#;
pub(crate) fn schema_digest() -> [u8; 32] {
    Sha256::digest(SCHEMA.as_bytes()).into()
}
pub(crate) fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub(crate) fn counter(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}
pub(crate) fn read_counter(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(
        bytes.try_into().map_err(|_| StoreError::Corrupt)?,
    ))
}
pub(crate) fn reserve(path: &Path) -> Result<()> {
    local_path(path)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                StoreError::AlreadyExists
            } else {
                StoreError::Io
            }
        })
}
fn local_path(path: &Path) -> Result<()> {
    // Mapped-drive classification remains the host's local-storage precondition.
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next()
        && !matches!(
            prefix.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
        )
    {
        return Err(StoreError::Io);
    }
    Ok(())
}
pub(crate) fn connection(path: &Path) -> Result<Connection> {
    local_path(path)?;
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(StoreError::from)?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 2_097_152);
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536);
    Ok(connection)
}
pub(crate) fn verify_existing(connection: &Connection) -> Result<()> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if id != APPLICATION_ID || version != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema);
    }
    let integrity: String =
        connection.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StoreError::Corrupt);
    }
    let extra: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))",
        [], |row| row.get(0),
    )?;
    if extra {
        return Err(StoreError::UnsupportedSchema);
    }
    let expected: std::collections::BTreeMap<_, _> = SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|sql| !sql.is_empty())
        .map(|sql| {
            let name = sql
                .strip_prefix("CREATE TABLE ")
                .and_then(|sql| sql.split_whitespace().next())
                .expect("static schema table");
            (name.to_owned(), sql.to_owned())
        })
        .collect();
    let mut statement=connection.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 11")?;
    let actual: std::collections::BTreeMap<String, String> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if actual != expected {
        return Err(StoreError::UnsupportedSchema);
    }
    Ok(())
}
pub(crate) fn configure(connection: &Connection) -> Result<()> {
    connection.busy_timeout(Duration::from_millis(250))?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 2_097_152);
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536);
    connection.set_limit(Limit::SQLITE_LIMIT_COLUMN, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128);
    connection.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    connection.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA locking_mode=EXCLUSIVE; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA cache_size=-2048; PRAGMA max_page_count=65536;")?;
    let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    let sync: i32 = connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    let locking: String = connection.pragma_query_value(None, "locking_mode", |row| row.get(0))?;
    let foreign: i32 = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    let trusted: i32 = connection.pragma_query_value(None, "trusted_schema", |row| row.get(0))?;
    if journal != "delete" || sync != 3 || locking != "exclusive" || foreign != 1 || trusted != 0 {
        return Err(StoreError::Io);
    }
    let pages: u32 = connection.pragma_query_value(None, "max_page_count", |row| row.get(0))?;
    let page_size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    if pages > 65536 || page_size != 4096 {
        return Err(StoreError::Limit);
    }
    connection.execute_batch("BEGIN EXCLUSIVE; COMMIT;")?;
    Ok(())
}
