use super::{StageBinding, StageError, StageRecord, derive, membership_digest, record};
use crate::{
    ImmutableSnapshot, KnownFrontiers, Store, StoreError, codec, identity,
    model::MAX_STATE_BYTES,
    persistence::bounded_blob,
    schema,
};
use rusqlite::{Connection, DatabaseName, OpenFlags, TransactionBehavior, limits::Limit, params};
use std::{collections::BTreeMap, fs::OpenOptions, path::Path};
const APPLICATION_ID: i32 = 0x4e46_5347;
const VERSION: i32 = 1;
const MAX_FILE: u64 = 268_435_456;
const TABLE: &str = "CREATE TABLE migration_stage (singleton INTEGER PRIMARY KEY CHECK(singleton=1), transition BLOB NOT NULL CHECK(length(transition)<=4096), candidate_state BLOB NOT NULL CHECK(length(candidate_state)<=1048576), candidate_digest BLOB NOT NULL CHECK(length(candidate_digest)=32)) STRICT";

pub(crate) fn write(
    source: &Store,
    path: &Path,
    metadata: &[u8],
    candidate: &[u8],
) -> Result<(), StageError> {
    let pages: u32 = source
        .connection
        .pragma_query_value(None, "max_page_count", |row| row.get(0))?;
    if pages == 0 || pages > 65536 {
        return Err(StageError::Storage(StoreError::Limit));
    }
    // All source/plan/state guards are complete before this sole output reservation.
    schema::reserve(path)?;
    source.connection.backup(DatabaseName::Main, path, None)?;
    let mut connection = schema::connection(path)?;
    schema::configure(&connection)?;
    connection.pragma_update(None, "max_page_count", pages)?;
    let actual: u32 = connection.pragma_query_value(None, "max_page_count", |row| row.get(0))?;
    if actual != pages {
        return Err(StageError::Storage(StoreError::Limit));
    }
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(TABLE)?;
    transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    transaction.pragma_update(None, "user_version", VERSION)?;
    transaction.execute(
        "INSERT INTO migration_stage VALUES (1,?1,?2,?3)",
        params![metadata, candidate, schema::hash(candidate)],
    )?;
    transaction.commit()?;
    drop(connection);
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| StageError::Storage(StoreError::Io))?
        .sync_all()
        .map_err(|_| StageError::Storage(StoreError::Io))?;
    if std::fs::metadata(path)
        .map_err(|_| StageError::Storage(StoreError::Io))?
        .len()
        > MAX_FILE
    {
        return Err(StageError::Storage(StoreError::Limit));
    }
    Ok(())
}
fn readonly(path: &Path) -> Result<Connection, StageError> {
    // Mapped-drive classification, like ordinary Profile1, remains a host precondition.
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next()
        && !matches!(
            prefix.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
        )
    {
        return Err(StageError::Storage(StoreError::Io));
    }
    let size = std::fs::metadata(path)
        .map_err(|_| StageError::Storage(StoreError::Io))?
        .len();
    if !(100..=MAX_FILE).contains(&size) {
        return Err(StageError::Storage(StoreError::Limit));
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 2_097_152);
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536);
    connection.set_limit(Limit::SQLITE_LIMIT_COLUMN, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128);
    connection.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    connection.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA cache_size=-2048;")?;
    Ok(connection)
}
fn layout(connection: &Connection) -> Result<(), StageError> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if id != APPLICATION_ID || version != VERSION {
        return Err(StageError::UnsupportedFormat);
    }
    let page_size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    let page_count: u32 = connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
    if page_size != 4096 || page_count > 65536 {
        return Err(StageError::Storage(StoreError::Limit));
    }
    let integrity: String =
        connection.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StageError::Corrupt);
    }
    let extra: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))",
        [],
        |row| row.get(0),
    )?;
    if extra {
        return Err(StageError::UnsupportedFormat);
    }
    let mut expected = BTreeMap::new();
    for sql in schema::SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .chain([TABLE])
    {
        let name = sql
            .strip_prefix("CREATE TABLE ")
            .and_then(|sql| sql.split_whitespace().next())
            .ok_or(StageError::Corrupt)?;
        expected.insert(name.to_owned(), sql.to_owned());
    }
    let mut statement = connection.prepare(
        "SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 12",
    )?;
    let actual: BTreeMap<String, String> = statement
        .query_map([], |row| {
            let name = row.get_ref(0)?.as_str()?;
            let sql = row.get_ref(1)?.as_str()?;
            if name.len() > 128 || sql.len() > 16384 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            Ok((name.to_owned(), sql.to_owned()))
        })?
        .collect::<rusqlite::Result<_>>()?;
    if actual != expected {
        return Err(StageError::UnsupportedFormat);
    }
    Ok(())
}
pub(crate) fn verify(path: &Path, expected: &StageBinding) -> Result<StageRecord, StageError> {
    let connection = readonly(path)?;
    let transaction = connection.unchecked_transaction()?;
    layout(&transaction)?;
    let count: i64 = transaction.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM migration_stage LIMIT 2)",
        [],
        |row| row.get(0),
    )?;
    if count != 1 {
        return Err(StageError::Corrupt);
    }
    let (metadata, body, digest) = transaction.query_row(
        "SELECT transition,candidate_state,candidate_digest FROM migration_stage WHERE singleton=1",
        [],
        |row| {
            Ok((
                bounded_blob(row, 0, record::MAX_BYTES)?,
                bounded_blob(row, 1, MAX_STATE_BYTES)?,
                bounded_blob(row, 2, 32)?,
            ))
        },
    )?;
    let plan = record::decode(&metadata)?;
    if &plan.binding != expected || schema::hash(&body).as_slice() != digest.as_slice() {
        return Err(StageError::SourceMismatch);
    }
    // Ordinary Store opening must refuse NFSG; recover is used only after our exact layout gate.
    let (source, revision, head) = crate::recovery::recover(&transaction, expected.known)?;
    let membership = identity::load(&transaction, expected.known.scope)?;
    let known = KnownFrontiers {
        scope: source.scope(),
        event_sequence: source.world.view().event_seq(),
        store_revision: revision,
        membership_revision: membership.as_ref().map(|m| m.revision),
    };
    if known != expected.known
        || head != expected.head
        || schema::hash(&codec::encode(&source)?) != expected.snapshot
        || membership_digest(membership.as_ref())? != expected.membership
    {
        return Err(StageError::SourceMismatch);
    }
    let candidate = derive(&source, &plan)?;
    let snapshot = ImmutableSnapshot::from_state(revision, &candidate)?;
    // Decode/validation is bounded, but only this independent derivation establishes the relation.
    codec::decode(&body)?;
    if snapshot.bytes() != body.as_slice() {
        return Err(StageError::DerivationMismatch);
    }
    Ok(StageRecord {
        binding: *expected,
        candidate,
        snapshot,
        membership,
    })
}
