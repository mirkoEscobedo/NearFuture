use super::{
    bytes::hash,
    error::{ManagedError, Result},
};
use rusqlite::{Connection, OpenFlags, limits::Limit};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::Read,
    path::{Component, Path, PathBuf},
    time::Duration,
};

pub(super) const ACTIVE_SCHEMA: &str = r#"
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
CREATE TABLE active_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), membership_revision BLOB CHECK(membership_revision IS NULL OR length(membership_revision)=8), quota BLOB NOT NULL CHECK(length(quota)=4), state BLOB NOT NULL CHECK(length(state)<=1884160), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), lineage_digest BLOB NOT NULL CHECK(length(lineage_digest)=32), checkpoint_revision BLOB NOT NULL CHECK(length(checkpoint_revision)=8), checkpoint_digest BLOB NOT NULL CHECK(length(checkpoint_digest)=32), checkpoint_head BLOB NOT NULL CHECK(length(checkpoint_head)=32)) STRICT;
CREATE TABLE active_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 7), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=1114112), state BLOB NOT NULL CHECK(length(state)<=1884160)) STRICT;
CREATE TABLE active_events (sequence BLOB PRIMARY KEY CHECK(length(sequence)=8), revision BLOB NOT NULL UNIQUE CHECK(length(revision)=8), body BLOB NOT NULL CHECK(length(body)<=1048576), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE active_snapshots (revision BLOB PRIMARY KEY CHECK(length(revision)=8), event_seq BLOB NOT NULL CHECK(length(event_seq)=8), membership_revision BLOB CHECK(membership_revision IS NULL OR length(membership_revision)=8), quota BLOB NOT NULL CHECK(length(quota)=4), state BLOB NOT NULL CHECK(length(state)<=1884160), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), lineage_digest BLOB NOT NULL CHECK(length(lineage_digest)=32)) STRICT;
CREATE TABLE migration_lineage (singleton INTEGER PRIMARY KEY CHECK(singleton=1), record BLOB NOT NULL CHECK(length(record)<=16384), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
"#;
pub(super) const RETIRED_SCHEMA: &str = r#"
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
CREATE TABLE owner_retirement (singleton INTEGER PRIMARY KEY CHECK(singleton=1), record BLOB NOT NULL CHECK(length(record)<=8192), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
"#;
pub(super) const CATALOG_SCHEMA: &str = r#"CREATE TABLE catalog_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), state BLOB NOT NULL CHECK(length(state)<=16384), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32)) STRICT;
CREATE TABLE catalog_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 8), previous BLOB NOT NULL CHECK(length(previous)=32), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), action BLOB NOT NULL CHECK(length(action)<=131072), state BLOB NOT NULL CHECK(length(state)<=16384)) STRICT;
CREATE TABLE catalog_snapshots (revision BLOB PRIMARY KEY CHECK(length(revision)=8), state BLOB NOT NULL CHECK(length(state)<=16384), digest BLOB NOT NULL CHECK(length(digest)=32), head BLOB NOT NULL CHECK(length(head)=32), rows BLOB NOT NULL CHECK(length(rows)<=1179648)) STRICT;
CREATE TABLE pending_world (singleton INTEGER PRIMARY KEY CHECK(singleton=1), sequence BLOB NOT NULL CHECK(length(sequence)=8), operation BLOB NOT NULL UNIQUE CHECK(length(operation)=16), stage INTEGER NOT NULL CHECK(stage BETWEEN 1 AND 2), record BLOB NOT NULL CHECK(length(record)<=5242880), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE operation_receipts (sequence BLOB PRIMARY KEY CHECK(length(sequence)=8), operation BLOB NOT NULL UNIQUE CHECK(length(operation)=16), result BLOB NOT NULL CHECK(length(result)<=4096), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;
CREATE TABLE artifact_registry (kind INTEGER PRIMARY KEY CHECK(kind BETWEEN 1 AND 5), artifact BLOB NOT NULL UNIQUE CHECK(length(artifact)=16), path BLOB NOT NULL CHECK(length(path)<=4096), bytes BLOB NOT NULL CHECK(length(bytes)=8), digest BLOB NOT NULL CHECK(length(digest)=32), manifest BLOB NOT NULL CHECK(length(manifest)<=8192)) STRICT;
"#;

pub fn active_schema_digest() -> [u8; 32] {
    hash(ACTIVE_SCHEMA.as_bytes())
}
pub fn retired_schema_digest() -> [u8; 32] {
    hash(RETIRED_SCHEMA.as_bytes())
}
pub fn catalog_schema_digest() -> [u8; 32] {
    hash(CATALOG_SCHEMA.as_bytes())
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Profile {
    Source,
    Active,
    Retired,
    Catalog,
}
impl Profile {
    pub fn id(self) -> i32 {
        if self == Self::Catalog {
            0x4e46_4f43
        } else {
            0x4e46_5354
        }
    }
    pub fn version(self) -> u32 {
        match self {
            Self::Source | Self::Catalog => 1,
            Self::Active => 4,
            Self::Retired => 5,
        }
    }
    pub fn pages(self) -> u32 {
        if self == Self::Catalog {
            16_384
        } else {
            65_536
        }
    }
    pub fn row_cap(self) -> i32 {
        match self {
            Self::Catalog => 8_388_608,
            Self::Source => 2_097_152,
            _ => 4_194_304,
        }
    }
    pub fn ddl(self) -> &'static str {
        match self {
            Self::Source => crate::schema::SCHEMA,
            Self::Active => ACTIVE_SCHEMA,
            Self::Retired => RETIRED_SCHEMA,
            Self::Catalog => CATALOG_SCHEMA,
        }
    }
}
pub(super) fn local_absolute(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(ManagedError::Storage(crate::StoreError::Io));
    }
    if let Some(Component::Prefix(p)) = path.components().next() {
        if !matches!(
            p.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
        ) {
            return Err(ManagedError::Storage(crate::StoreError::Io));
        }
    }
    let text = path.to_str().ok_or(ManagedError::Scope)?;
    if text.len() > 4096 || text.as_bytes().contains(&0) {
        return Err(ManagedError::Limit);
    }
    let parent = path.parent().ok_or(ManagedError::Scope)?;
    let canonical = parent.canonicalize()?;
    if canonical != parent || !std::fs::symlink_metadata(parent)?.is_dir() {
        return Err(ManagedError::Scope);
    }
    Ok(path.to_path_buf())
}
pub(super) fn regular_file(meta: &std::fs::Metadata) -> Result<()> {
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(ManagedError::UnsupportedFormat);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(ManagedError::UnsupportedFormat);
        }
    }
    Ok(())
}
pub(super) fn companion(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
pub(super) fn absent(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(ManagedError::AlreadyExists),
        Err(error) => Err(error.into()),
    }
}
pub(super) fn companions_absent(path: &Path, allow_owned_journal: bool) -> Result<()> {
    if allow_owned_journal {
        return super::journal::companion(path, true);
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        absent(&companion(path, suffix))?;
    }
    Ok(())
}
pub(super) fn reserve(path: &Path) -> Result<File> {
    local_absolute(path)?;
    absent(path)?;
    companions_absent(path, false)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                ManagedError::AlreadyExists
            } else {
                e.into()
            }
        })
}
pub(super) fn header(path: &Path, profile: Profile) -> Result<()> {
    header_selected(path, &[profile])
}
pub(super) fn header_selected(path: &Path, profiles: &[Profile]) -> Result<()> {
    let maximum = profiles
        .iter()
        .map(|p| p.pages())
        .max()
        .ok_or(ManagedError::UnsupportedFormat)?;
    local_absolute(path)?;
    let meta = std::fs::symlink_metadata(path)?;
    regular_file(&meta)?;
    let mut file = File::open(path)?;
    let opened = file.metadata()?;
    if opened.len() != meta.len() || opened.len() < 100 || opened.len() > u64::from(maximum) * 4096
    {
        return Err(ManagedError::Limit);
    }
    let mut h = [0_u8; 100];
    file.read_exact(&mut h)?;
    if &h[..16] != b"SQLite format 3\0"
        || u16::from_be_bytes([h[16], h[17]]) != 4096
        || !profiles.iter().any(|p| {
            u32::from_be_bytes([h[60], h[61], h[62], h[63]]) == p.version()
                && i32::from_be_bytes([h[68], h[69], h[70], h[71]]) == p.id()
        })
        || h[18] != 1
        || h[19] != 1
    {
        return Err(ManagedError::UnsupportedFormat);
    }
    Ok(())
}
pub(super) fn limits(c: &Connection, profile: Profile) -> Result<()> {
    c.busy_timeout(Duration::from_millis(250))?;
    for (limit, value) in [
        (Limit::SQLITE_LIMIT_LENGTH, profile.row_cap()),
        (Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536),
        (Limit::SQLITE_LIMIT_COLUMN, 64),
        (Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128),
        (Limit::SQLITE_LIMIT_EXPR_DEPTH, 64),
        (Limit::SQLITE_LIMIT_ATTACHED, 0),
    ] {
        c.set_limit(limit, value);
    }
    c.pragma_update(None, "trusted_schema", false)?;
    Ok(())
}
pub(super) fn physical(c: &Connection, profile: Profile) -> Result<()> {
    let id: i32 = c.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let version: u32 = c.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if id != profile.id() || version != profile.version() {
        return Err(ManagedError::UnsupportedFormat);
    }
    let expected: BTreeMap<String, String> = profile
        .ddl()
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let name = s
                .strip_prefix("CREATE TABLE ")
                .and_then(|s| s.split_whitespace().next())
                .ok_or(ManagedError::Corrupt)?;
            Ok((name.to_string(), s.to_string()))
        })
        .collect::<Result<_>>()?;
    let mut actual = BTreeMap::new();
    let mut statement =
        c.prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY name LIMIT 65")?;
    let mut rows = statement.query([])?;
    let mut seen = 0;
    while let Some(row) = rows.next()? {
        seen += 1;
        if seen > 64 {
            return Err(ManagedError::Limit);
        }
        let kind: String = row.get(0)?;
        let name: String = row.get(1)?;
        let table: String = row.get(2)?;
        let sql: Option<String> = row.get(3)?;
        if name.len() > 256 || table.len() > 256 || sql.as_ref().is_some_and(|s| s.len() > 65_536) {
            return Err(ManagedError::Limit);
        }
        if kind == "table" {
            if actual
                .insert(name, sql.ok_or(ManagedError::Corrupt)?)
                .is_some()
            {
                return Err(ManagedError::UnsupportedFormat);
            }
        } else if kind == "index" && sql.is_none() {
            let definition = expected
                .get(&table)
                .ok_or(ManagedError::UnsupportedFormat)?;
            let primary = usize::from(
                definition.contains("BLOB PRIMARY KEY")
                    || definition.contains("PRIMARY KEY(universe,history)"),
            );
            let count = primary + definition.matches(" UNIQUE").count();
            if !(1..=count).any(|n| name == format!("sqlite_autoindex_{table}_{n}")) {
                return Err(ManagedError::UnsupportedFormat);
            }
        } else {
            return Err(ManagedError::UnsupportedFormat);
        }
    }
    if actual != expected {
        return Err(ManagedError::UnsupportedFormat);
    }
    let integrity: String = c.query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(ManagedError::Corrupt);
    }
    Ok(())
}
pub(super) fn configure(c: &Connection, profile: Profile, quota: u32) -> Result<()> {
    limits(c, profile)?;
    if quota == 0 || quota > profile.pages() {
        return Err(ManagedError::Limit);
    }
    let admitted_size: u32 = c.pragma_query_value(None, "page_size", |r| r.get(0))?;
    let admitted_journal: String = c.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
    if admitted_size != 4096 || admitted_journal != "delete" {
        return Err(ManagedError::UnsupportedFormat);
    }
    c.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA; PRAGMA locking_mode=EXCLUSIVE; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA cache_size=-2048;")?;
    c.pragma_update(None, "max_page_count", quota)?;
    let journal: String = c.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
    let sync: i32 = c.pragma_query_value(None, "synchronous", |r| r.get(0))?;
    let locking: String = c.pragma_query_value(None, "locking_mode", |r| r.get(0))?;
    let foreign: i32 = c.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
    let trusted: i32 = c.pragma_query_value(None, "trusted_schema", |r| r.get(0))?;
    let pages: u32 = c.pragma_query_value(None, "page_count", |r| r.get(0))?;
    let max: u32 = c.pragma_query_value(None, "max_page_count", |r| r.get(0))?;
    let size: u32 = c.pragma_query_value(None, "page_size", |r| r.get(0))?;
    if journal != "delete"
        || sync != 3
        || locking != "exclusive"
        || foreign != 1
        || trusted != 0
        || max != quota
        || pages > quota
        || size != 4096
    {
        return Err(ManagedError::Storage(crate::StoreError::Io));
    }
    c.execute_batch("BEGIN EXCLUSIVE; COMMIT;")?;
    Ok(())
}
pub(super) fn open(
    path: &Path,
    profile: Profile,
    quota: u32,
    journal: super::journal::Admission,
) -> Result<Connection> {
    super::journal::admit(path, journal)?;
    header(path, profile)?;
    let c = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    limits(&c, profile)?;
    physical(&c, profile)?;
    configure(&c, profile, quota)?;
    Ok(c)
}
pub(super) fn close_sync(c: Connection, path: &Path) -> Result<()> {
    c.close().map_err(|(_, e)| ManagedError::from(e))?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()?;
    Ok(())
}
