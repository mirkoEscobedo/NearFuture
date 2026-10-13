use super::{
    ConfigUpgradeTransition, PreparedConfigUpgrade, UpgradeError, derive, profile3_schema_digest,
    record,
};
use crate::{
    ImmutableSnapshot, KnownFrontiers, Store, StoreError, codec, identity,
    model::MAX_STATE_BYTES,
    persistence::bounded_blob,
    schema,
    staging::{StageBinding, membership_digest},
};
use rusqlite::{
    Connection, OpenFlags, TransactionBehavior,
    backup::{Backup, StepResult},
    limits::Limit,
    params,
};
use std::{collections::BTreeMap, fs::OpenOptions, path::Path, time::Duration};
const APPLICATION_ID: i32 = 0x4e46_5354;
const VERSION: i32 = 3;
const MAX_FILE: u64 = 268_435_456;
pub(crate) const DDL: &str = "CREATE TABLE config_upgrade_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), state BLOB NOT NULL CHECK(length(state)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32)) STRICT;\nCREATE TABLE config_upgrade_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 4), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=4096), state BLOB NOT NULL CHECK(length(state)<=1048576)) STRICT;\n";

fn companions_absent(path: &Path, occupied: UpgradeError) -> Result<(), UpgradeError> {
    // Owned-parent custody only; no companion is opened, followed, removed or overwritten.
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut companion = path.as_os_str().to_os_string();
        companion.push(suffix);
        match std::fs::symlink_metadata(Path::new(&companion)) {
            Ok(_) => return Err(occupied),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(UpgradeError::Storage(StoreError::Io)),
        }
    }
    Ok(())
}
fn limits(connection: &Connection) -> Result<(), UpgradeError> {
    connection.busy_timeout(Duration::from_millis(250))?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 2_097_152);
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536);
    connection.set_limit(Limit::SQLITE_LIMIT_COLUMN, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128);
    connection.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    Ok(())
}
fn source_limits(source: &Store) -> Result<(u32, u32), UpgradeError> {
    let connection = &source.connection;
    let path = connection.path().ok_or(StoreError::Io)?;
    let size = std::fs::metadata(path).map_err(|_| StoreError::Io)?.len();
    let quota: u32 = connection.pragma_query_value(None, "max_page_count", |row| row.get(0))?;
    let pages: u32 = connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
    let page_size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if !(100..=MAX_FILE).contains(&size)
        || quota == 0
        || quota > 65536
        || pages == 0
        || pages > quota
        || page_size != 4096
    {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    if journal != "delete" {
        return Err(UpgradeError::UnsupportedFormat);
    }
    Ok((quota, pages))
}
fn backup(source: &Connection, target: &mut Connection, pages: u32) -> Result<(), UpgradeError> {
    let backup = Backup::new(source, target)?;
    // Done includes the final copy step. At most ceil(source_pages/100) calls (maximum 656).
    // A changing source cannot obtain an extra completion step or Busy/Locked retry.
    for _ in 0..pages.div_ceil(100) {
        match backup.step(100)? {
            StepResult::Done => return Ok(()),
            StepResult::More => {}
            StepResult::Busy | StepResult::Locked => {
                return Err(UpgradeError::Storage(StoreError::Busy));
            }
            _ => return Err(UpgradeError::Storage(StoreError::Io)),
        }
    }
    Err(UpgradeError::Storage(StoreError::Limit))
}
pub(crate) fn write(
    source: &Store,
    path: &Path,
    transition: &ConfigUpgradeTransition,
    action: &[u8],
    snapshot: &ImmutableSnapshot,
) -> Result<(), UpgradeError> {
    let (quota, pages) = source_limits(source)?;
    if action.len() > record::MAX_BYTES || snapshot.bytes().len() > MAX_STATE_BYTES {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    // Two new bounded payloads, their leaves, and conservative schema/root/index room.
    // 3837 is the overflow payload even with the maximum 255 reserved bytes per 4096 page.
    let state_bytes = snapshot.bytes().len() as u64;
    let action_bytes = action.len() as u64;
    let metadata_pages = (state_bytes + 256).div_ceil(3837) + 1;
    let journal_pages = (state_bytes + action_bytes + 256).div_ceil(3837) + 1;
    let required = 12 + metadata_pages + journal_pages;
    if u64::from(quota - pages) < required {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    // Source/binding/transition/revision/state/quota/companion guards precede CreateNew.
    companions_absent(path, UpgradeError::Storage(StoreError::AlreadyExists))?;
    schema::reserve(path)?;
    let mut connection = schema::connection(path)?;
    limits(&connection)?;
    backup(&source.connection, &mut connection, pages)?;
    schema::verify_existing(&connection)?;
    schema::configure(&connection)?;
    connection.pragma_update(None, "max_page_count", quota)?;
    let actual: u32 = connection.pragma_query_value(None, "max_page_count", |row| row.get(0))?;
    if actual != quota {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    let head = record::head(
        transition.binding.head,
        snapshot.revision(),
        action,
        transition.binding.snapshot,
        snapshot.digest(),
    );
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(DDL)?;
    transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    transaction.pragma_update(None, "user_version", VERSION)?;
    transaction.execute(
        "INSERT INTO config_upgrade_journal VALUES (?1,4,?2,?3,?4,?5,?6)",
        params![
            schema::counter(snapshot.revision()),
            transition.binding.snapshot,
            snapshot.digest(),
            head,
            action,
            snapshot.bytes()
        ],
    )?;
    transaction.execute(
        "INSERT INTO config_upgrade_meta VALUES (1,?1,?2,?3,?4,?5,?6)",
        params![
            schema::counter(snapshot.revision()),
            schema::counter(transition.binding.known.event_sequence.0),
            snapshot.bytes(),
            snapshot.digest(),
            head,
            profile3_schema_digest()
        ],
    )?;
    transaction.commit()?;
    connection
        .close()
        .map_err(|(_, error)| UpgradeError::from(error))?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| StoreError::Io)?
        .sync_all()
        .map_err(|_| StoreError::Io)?;
    if std::fs::metadata(path).map_err(|_| StoreError::Io)?.len() > MAX_FILE {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    Ok(())
}
fn readonly(path: &Path) -> Result<Connection, UpgradeError> {
    // Mapped-drive classification and hostile-path exclusion remain host preconditions.
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next()
        && !matches!(
            prefix.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
        )
    {
        return Err(UpgradeError::Storage(StoreError::Io));
    }
    companions_absent(path, UpgradeError::UnsupportedFormat)?;
    let size = std::fs::metadata(path).map_err(|_| StoreError::Io)?.len();
    if !(100..=MAX_FILE).contains(&size) {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    limits(&connection)?;
    connection.execute_batch(
        "PRAGMA trusted_schema=OFF; PRAGMA query_only=ON; PRAGMA cache_size=-2048;",
    )?;
    Ok(connection)
}
fn layout(connection: &Connection) -> Result<(), UpgradeError> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if id != APPLICATION_ID || version != VERSION {
        return Err(UpgradeError::UnsupportedFormat);
    }
    let page_size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    let page_count: u32 = connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
    let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if page_size != 4096 || page_count == 0 || page_count > 65536 {
        return Err(UpgradeError::Storage(StoreError::Limit));
    }
    if journal != "delete" {
        return Err(UpgradeError::UnsupportedFormat);
    }
    let integrity: String =
        connection.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(UpgradeError::Corrupt);
    }
    let extra: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))",
        [],
        |row| row.get(0),
    )?;
    if extra {
        return Err(UpgradeError::UnsupportedFormat);
    }
    let mut expected = BTreeMap::new();
    for sql in schema::SCHEMA
        .split(';')
        .chain(DDL.split(';'))
        .map(str::trim)
        .filter(|sql| !sql.is_empty())
    {
        let name = sql
            .strip_prefix("CREATE TABLE ")
            .and_then(|sql| sql.split_whitespace().next())
            .ok_or(UpgradeError::Corrupt)?;
        expected.insert(name.to_owned(), sql.to_owned());
    }
    let mut statement = connection.prepare(
        "SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 13",
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
        return Err(UpgradeError::UnsupportedFormat);
    }
    Ok(())
}
struct Journal {
    revision: Vec<u8>,
    kind: u32,
    previous: Vec<u8>,
    digest: Vec<u8>,
    head: Vec<u8>,
    action: Vec<u8>,
    state: Vec<u8>,
}
struct Metadata {
    revision: Vec<u8>,
    event: Vec<u8>,
    state: Vec<u8>,
    digest: Vec<u8>,
    head: Vec<u8>,
    schema: Vec<u8>,
}
fn rows(connection: &Connection) -> Result<(Journal, Metadata), UpgradeError> {
    let journal_count: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM config_upgrade_journal LIMIT 2)",
        [],
        |row| row.get(0),
    )?;
    let meta_count: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM config_upgrade_meta LIMIT 2)",
        [],
        |row| row.get(0),
    )?;
    if journal_count != 1 || meta_count != 1 {
        return Err(UpgradeError::Corrupt);
    }
    let journal = connection.query_row(
        "SELECT revision,kind,previous,digest,head,action,state FROM config_upgrade_journal",
        [],
        |row| {
            Ok(Journal {
                revision: bounded_blob(row, 0, 8)?,
                kind: row.get(1)?,
                previous: bounded_blob(row, 2, 32)?,
                digest: bounded_blob(row, 3, 32)?,
                head: bounded_blob(row, 4, 32)?,
                action: bounded_blob(row, 5, record::MAX_BYTES)?,
                state: bounded_blob(row, 6, MAX_STATE_BYTES)?,
            })
        },
    )?;
    let metadata = connection.query_row(
        "SELECT revision,event_seq,state,digest,head,schema_digest FROM config_upgrade_meta WHERE singleton=1",
        [],
        |row| {
            Ok(Metadata {
                revision: bounded_blob(row, 0, 8)?,
                event: bounded_blob(row, 1, 8)?,
                state: bounded_blob(row, 2, MAX_STATE_BYTES)?,
                digest: bounded_blob(row, 3, 32)?,
                head: bounded_blob(row, 4, 32)?,
                schema: bounded_blob(row, 5, 32)?,
            })
        },
    )?;
    Ok((journal, metadata))
}
fn minimum(known: KnownFrontiers, expected: KnownFrontiers) -> Result<(), UpgradeError> {
    if known.scope != expected.scope {
        return Err(UpgradeError::Storage(StoreError::Scope));
    }
    if known.store_revision < expected.store_revision
        || known.event_sequence < expected.event_sequence
        || expected
            .membership_revision
            .is_some_and(|minimum| known.membership_revision.is_none_or(|value| value < minimum))
    {
        return Err(UpgradeError::Storage(StoreError::StaleBackup));
    }
    Ok(())
}
pub(crate) fn verify(
    path: &Path,
    expected: &StageBinding,
    intended_target: [u8; 32],
    protected: KnownFrontiers,
) -> Result<PreparedConfigUpgrade, UpgradeError> {
    let connection = readonly(path)?;
    let transaction = connection.unchecked_transaction()?;
    layout(&transaction)?;
    let (journal, metadata) = rows(&transaction)?;
    let plan = record::decode(&journal.action)?;
    if &plan.binding != expected {
        return Err(UpgradeError::SourceMismatch);
    }
    // Only the new NFST3 layout gate grants access to unchanged base replay.
    // TargetR+1 minima are checked after the baseR replay, never substituted here.
    let (source, revision, head) = crate::recovery::recover(&transaction, expected.known)?;
    let membership = identity::load(&transaction, expected.known.scope)?;
    let source_known = KnownFrontiers {
        scope: source.scope(),
        event_sequence: source.world.view().event_seq(),
        store_revision: revision,
        membership_revision: membership.as_ref().map(|state| state.revision),
    };
    let source_digest = schema::hash(&codec::encode(&source)?);
    let actual = StageBinding {
        known: source_known,
        head,
        snapshot: source_digest,
        membership: membership_digest(membership.as_ref())?,
    };
    if &actual != expected {
        return Err(UpgradeError::SourceMismatch);
    }
    let revision = revision.checked_add(1).ok_or(StoreError::Limit)?;
    let candidate = derive(&source, &plan)?;
    if candidate.world.to_spec().ruleset_hash != intended_target {
        return Err(UpgradeError::SourceMismatch);
    }
    let snapshot = ImmutableSnapshot::from_state(revision, &candidate)?;
    let expected_head = record::head(
        head,
        revision,
        &journal.action,
        source_digest,
        snapshot.digest(),
    );
    if journal.kind != 4
        || schema::read_counter(&journal.revision)? != revision
        || journal.previous != source_digest
        || journal.digest != snapshot.digest()
        || journal.head != expected_head
        || journal.state.as_slice() != snapshot.bytes()
        || metadata.revision != journal.revision
        || schema::read_counter(&metadata.event)? != source_known.event_sequence.0
        || metadata.state != journal.state
        || metadata.digest != journal.digest
        || metadata.head != journal.head
        || metadata.schema != profile3_schema_digest()
    {
        return Err(UpgradeError::DerivationMismatch);
    }
    let known = KnownFrontiers {
        store_revision: revision,
        ..source_known
    };
    minimum(known, protected)?;
    Ok(PreparedConfigUpgrade {
        binding: actual,
        candidate,
        snapshot,
        membership,
        known,
    })
}
