use super::{
    bytes::hash,
    error::{ManagedError, Result},
    schema::Profile,
};
use rusqlite::{
    Connection, params_from_iter,
    types::{Value, ValueRef},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const ROW_CAP: u64 = 1_048_576;
const BYTE_CAP: u64 = 268_435_456;
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum Cell {
    Null,
    Integer(i64),
    Blob(Vec<u8>),
}
pub(super) type Row = Vec<Cell>;
pub(super) type Key = Vec<Cell>;
#[derive(Clone, Copy)]
pub(super) struct Table {
    pub name: &'static str,
    pub columns: usize,
    pub keys: &'static [&'static str],
}
pub(super) fn table(name: &str) -> Result<Table> {
    let (name, columns, keys): (&'static str, usize, &'static [&'static str]) = match name {
        "history_meta" => ("history_meta", 9, &["singleton"]),
        "snapshots" => ("snapshots", 4, &["revision"]),
        "journal" => ("journal", 7, &["revision"]),
        "events" => ("events", 3, &["sequence"]),
        "aggregate_state" => ("aggregate_state", 2, &["aggregate_id"]),
        "module_state" => ("module_state", 3, &["provider_id"]),
        "request_outcomes" => ("request_outcomes", 7, &["request_id"]),
        "reservations" => ("reservations", 3, &["operation_id"]),
        "outbox" => ("outbox", 4, &["operation_id"]),
        "membership" => ("membership", 5, &["universe", "history"]),
        "active_meta" => ("active_meta", 13, &["singleton"]),
        "active_journal" => ("active_journal", 7, &["revision"]),
        "active_events" => ("active_events", 4, &["sequence"]),
        "active_snapshots" => ("active_snapshots", 8, &["revision"]),
        "migration_lineage" => ("migration_lineage", 3, &["singleton"]),
        "owner_retirement" => ("owner_retirement", 3, &["singleton"]),
        "catalog_meta" => ("catalog_meta", 6, &["singleton"]),
        "catalog_journal" => ("catalog_journal", 7, &["revision"]),
        "catalog_snapshots" => ("catalog_snapshots", 5, &["revision"]),
        "pending_world" => ("pending_world", 6, &["singleton"]),
        "operation_receipts" => ("operation_receipts", 4, &["sequence"]),
        "artifact_registry" => ("artifact_registry", 6, &["kind"]),
        _ => return Err(ManagedError::UnsupportedFormat),
    };
    Ok(Table {
        name,
        columns,
        keys,
    })
}
pub(super) fn tables(profile: Profile) -> Result<Vec<Table>> {
    let mut out = Vec::new();
    for sql in profile
        .ddl()
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let name = sql
            .strip_prefix("CREATE TABLE ")
            .and_then(|s| s.split_whitespace().next())
            .ok_or(ManagedError::Corrupt)?;
        out.push(table(name)?);
    }
    out.sort_unstable_by_key(|t| t.name);
    Ok(out)
}
pub(super) fn key(row: &Row, table: Table) -> Result<Key> {
    if row.len() != table.columns
        || row[..table.keys.len()]
            .iter()
            .any(|v| matches!(v, Cell::Null))
    {
        return Err(ManagedError::Corrupt);
    }
    Ok(row[..table.keys.len()].to_vec())
}
fn values(row: &[Cell]) -> Vec<Value> {
    row.iter()
        .map(|c| match c {
            Cell::Null => Value::Null,
            Cell::Integer(n) => Value::Integer(*n),
            Cell::Blob(b) => Value::Blob(b.clone()),
        })
        .collect()
}
fn read_row(row: &rusqlite::Row<'_>, table: Table) -> Result<Row> {
    let mut out = Vec::with_capacity(table.columns);
    let mut bytes = 0_usize;
    for i in 0..table.columns {
        let cell = match row.get_ref(i)? {
            ValueRef::Null => Cell::Null,
            ValueRef::Integer(n) => Cell::Integer(n),
            ValueRef::Blob(b) => {
                bytes = bytes.checked_add(b.len()).ok_or(ManagedError::Limit)?;
                if bytes > 8_388_608 {
                    return Err(ManagedError::Limit);
                }
                Cell::Blob(b.to_vec())
            }
            _ => return Err(ManagedError::Corrupt),
        };
        out.push(cell);
    }
    Ok(out)
}
pub(super) fn get(c: &Connection, table: Table, key: &Key) -> Result<Option<Row>> {
    if key.len() != table.keys.len() {
        return Err(ManagedError::Corrupt);
    }
    let predicate = table
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k}=?{}", i + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let mut statement = c.prepare(&format!(
        "SELECT * FROM {} WHERE {predicate} LIMIT 2",
        table.name
    ))?;
    let args = values(key);
    let mut rows = statement.query(params_from_iter(args.iter()))?;
    let found = rows.next()?.map(|row| read_row(row, table)).transpose()?;
    if rows.next()?.is_some() {
        return Err(ManagedError::Corrupt);
    }
    Ok(found)
}
#[derive(Clone, Debug, Default)]
pub(super) struct Patch(pub BTreeMap<Key, (Option<Row>, Option<Row>)>);
impl Patch {
    pub fn put(&mut self, table: Table, before: Option<Row>, after: Option<Row>) -> Result<()> {
        let chosen = after
            .as_ref()
            .or(before.as_ref())
            .ok_or(ManagedError::Corrupt)?;
        let k = key(chosen, table)?;
        if before
            .as_ref()
            .map(|r| key(r, table))
            .transpose()?
            .is_some_and(|b| b != k)
            || self.0.len() >= ROW_CAP as usize
            || self.0.insert(k, (before, after)).is_some()
        {
            return Err(ManagedError::Corrupt);
        }
        Ok(())
    }
    pub fn apply(&self, c: &Connection, table: Table) -> Result<()> {
        for (k, (before, after)) in &self.0 {
            if get(c, table, k)? != *before {
                return Err(ManagedError::DerivationMismatch);
            }
            if before.is_some() {
                let predicate = table
                    .keys
                    .iter()
                    .enumerate()
                    .map(|(i, key)| format!("{key}=?{}", i + 1))
                    .collect::<Vec<_>>()
                    .join(" AND ");
                let args = values(k);
                if c.execute(
                    &format!("DELETE FROM {} WHERE {predicate}", table.name),
                    params_from_iter(args.iter()),
                )? != 1
                {
                    return Err(ManagedError::DerivationMismatch);
                }
            }
            if let Some(row) = after {
                let placeholders = (1..=table.columns)
                    .map(|i| format!("?{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let args = values(row);
                if c.execute(
                    &format!("INSERT INTO {} VALUES ({placeholders})", table.name),
                    params_from_iter(args.iter()),
                )? != 1
                {
                    return Err(ManagedError::DerivationMismatch);
                }
            }
        }
        Ok(())
    }
}
struct StreamHash {
    hash: Sha256,
    bytes: u64,
    rows: u64,
}
impl StreamHash {
    fn raw(&mut self, b: &[u8]) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(b.len().try_into().map_err(|_| ManagedError::Limit)?)
            .ok_or(ManagedError::Limit)?;
        if self.bytes > BYTE_CAP {
            return Err(ManagedError::Limit);
        }
        self.hash.update(b);
        Ok(())
    }
    fn row(&mut self, row: &Row, table: Table) -> Result<()> {
        key(row, table)?;
        self.rows = self.rows.checked_add(1).ok_or(ManagedError::Limit)?;
        if self.rows > ROW_CAP {
            return Err(ManagedError::Limit);
        }
        for cell in row {
            match cell {
                Cell::Null => self.raw(&[0])?,
                Cell::Integer(n) => {
                    self.raw(&[1])?;
                    self.raw(&n.to_le_bytes())?;
                }
                Cell::Blob(b) => {
                    self.raw(&[2])?;
                    self.raw(
                        &u32::try_from(b.len())
                            .map_err(|_| ManagedError::Limit)?
                            .to_le_bytes(),
                    )?;
                    self.raw(b)?;
                }
            }
        }
        Ok(())
    }
}
pub(super) fn digest(
    c: &Connection,
    tables: &[Table],
    patches: &BTreeMap<String, Patch>,
) -> Result<[u8; 32]> {
    digest_with_absent(c, tables, patches, &std::collections::BTreeSet::new())
}
pub(super) fn digest_with_absent(
    c: &Connection,
    tables: &[Table],
    patches: &BTreeMap<String, Patch>,
    absent: &std::collections::BTreeSet<&str>,
) -> Result<[u8; 32]> {
    if tables.windows(2).any(|p| p[0].name >= p[1].name) {
        return Err(ManagedError::Corrupt);
    }
    let mut h = StreamHash {
        hash: Sha256::new(),
        bytes: 0,
        rows: 0,
    };
    h.raw(b"NF-OWNER-ROWS-1\0\x01\0")?;
    h.raw(
        &u32::try_from(tables.len())
            .map_err(|_| ManagedError::Limit)?
            .to_le_bytes(),
    )?;
    for &table in tables {
        let n: i64 = if absent.contains(table.name) {
            0
        } else {
            c.query_row(
                &format!(
                    "SELECT count(*) FROM (SELECT 1 FROM {} LIMIT 1048577)",
                    table.name
                ),
                [],
                |r| r.get(0),
            )?
        };
        let mut count = u64::try_from(n).map_err(|_| ManagedError::Corrupt)?;
        if count > ROW_CAP {
            return Err(ManagedError::Limit);
        }
        let empty = Patch::default();
        let patch = patches.get(table.name).unwrap_or(&empty);
        for (k, (before, after)) in &patch.0 {
            if absent.contains(table.name) {
                if before.is_some() {
                    return Err(ManagedError::DerivationMismatch);
                }
            } else if get(c, table, k)? != *before {
                return Err(ManagedError::DerivationMismatch);
            }
            match (before.is_some(), after.is_some()) {
                (false, true) => count = count.checked_add(1).ok_or(ManagedError::Limit)?,
                (true, false) => count = count.checked_sub(1).ok_or(ManagedError::Corrupt)?,
                _ => {}
            }
        }
        h.raw(
            &u32::try_from(table.name.len())
                .map_err(|_| ManagedError::Limit)?
                .to_le_bytes(),
        )?;
        h.raw(table.name.as_bytes())?;
        h.raw(
            &u32::try_from(table.columns)
                .map_err(|_| ManagedError::Limit)?
                .to_le_bytes(),
        )?;
        h.raw(&count.to_be_bytes())?;
        if absent.contains(table.name) {
            let mut emitted = 0_u64;
            for (_, after) in patch.0.values() {
                if let Some(row) = after {
                    h.row(row, table)?;
                    emitted += 1;
                }
            }
            if emitted != count {
                return Err(ManagedError::Corrupt);
            }
            continue;
        }
        let mut statement = c.prepare(&format!(
            "SELECT * FROM {} ORDER BY {}",
            table.name,
            table.keys.join(",")
        ))?;
        let mut rows = statement.query([])?;
        let mut changes = patch.0.iter().peekable();
        let mut emitted = 0_u64;
        let mut previous = None;
        while let Some(row) = rows.next()? {
            let row = read_row(row, table)?;
            let k = key(&row, table)?;
            if previous.as_ref().is_some_and(|old| old >= &k) {
                return Err(ManagedError::Corrupt);
            }
            previous = Some(k.clone());
            while changes.peek().is_some_and(|(change, _)| *change < &k) {
                let (_, (_, after)) = changes.next().ok_or(ManagedError::Corrupt)?;
                if let Some(value) = after {
                    h.row(value, table)?;
                    emitted += 1;
                }
            }
            if changes.peek().is_some_and(|(change, _)| *change == &k) {
                let (_, (_, after)) = changes.next().ok_or(ManagedError::Corrupt)?;
                if let Some(value) = after {
                    h.row(value, table)?;
                    emitted += 1;
                }
            } else {
                h.row(&row, table)?;
                emitted += 1;
            }
        }
        for (_, (_, after)) in changes {
            if let Some(value) = after {
                h.row(value, table)?;
                emitted += 1;
            }
        }
        if count != emitted {
            return Err(ManagedError::Corrupt);
        }
    }
    Ok(h.hash.finalize().into())
}
pub(super) fn empty_digest(table: Table) -> Result<[u8; 32]> {
    let mut bytes = b"NF-OWNER-ROWS-1\0\x01\0".to_vec();
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(table.name.len())
            .map_err(|_| ManagedError::Limit)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(table.name.as_bytes());
    bytes.extend_from_slice(
        &u32::try_from(table.columns)
            .map_err(|_| ManagedError::Limit)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&0_u64.to_be_bytes());
    Ok(hash(&bytes))
}

pub(super) fn value_digest(table: Table, rows: &[Row]) -> Result<[u8; 32]> {
    let mut h = StreamHash {
        hash: Sha256::new(),
        bytes: 0,
        rows: 0,
    };
    h.raw(b"NF-OWNER-ROWS-1\0\x01\0")?;
    h.raw(&1_u32.to_le_bytes())?;
    h.raw(
        &u32::try_from(table.name.len())
            .map_err(|_| ManagedError::Limit)?
            .to_le_bytes(),
    )?;
    h.raw(table.name.as_bytes())?;
    h.raw(
        &u32::try_from(table.columns)
            .map_err(|_| ManagedError::Limit)?
            .to_le_bytes(),
    )?;
    h.raw(
        &u64::try_from(rows.len())
            .map_err(|_| ManagedError::Limit)?
            .to_be_bytes(),
    )?;
    let mut previous = None;
    for row in rows {
        let k = key(row, table)?;
        if previous.as_ref().is_some_and(|old| old >= &k) {
            return Err(ManagedError::Corrupt);
        }
        previous = Some(k);
        h.row(row, table)?;
    }
    Ok(h.hash.finalize().into())
}

pub(super) fn scan(c: &Connection, table: Table, cap: usize) -> Result<Vec<Row>> {
    if cap > 1_048_576 {
        return Err(ManagedError::Limit);
    }
    let mut statement = c.prepare(&format!(
        "SELECT * FROM {} ORDER BY {} LIMIT {}",
        table.name,
        table.keys.join(","),
        cap.checked_add(1).ok_or(ManagedError::Limit)?
    ))?;
    let mut rows = statement.query([])?;
    let mut out = Vec::new();
    let mut bytes = 0_usize;
    while let Some(row) = rows.next()? {
        if out.len() == cap {
            return Err(ManagedError::Limit);
        }
        let row = read_row(row, table)?;
        for cell in &row {
            let size = match cell {
                Cell::Null => 1,
                Cell::Integer(_) => 9,
                Cell::Blob(b) => b.len().checked_add(5).ok_or(ManagedError::Limit)?,
            };
            bytes = bytes.checked_add(size).ok_or(ManagedError::Limit)?;
            if bytes > BYTE_CAP as usize {
                return Err(ManagedError::Limit);
            }
        }
        if out.last().is_some_and(|previous| {
            key(previous, table).is_ok_and(|p| key(&row, table).is_ok_and(|k| p >= k))
        }) {
            return Err(ManagedError::Corrupt);
        }
        out.push(row);
    }
    Ok(out)
}
pub(super) fn blob(row: &Row, at: usize, cap: usize) -> Result<&[u8]> {
    match row.get(at) {
        Some(Cell::Blob(bytes)) if bytes.len() <= cap => Ok(bytes),
        _ => Err(ManagedError::Corrupt),
    }
}
pub(super) fn counter(row: &Row, at: usize) -> Result<u64> {
    Ok(u64::from_be_bytes(
        blob(row, at, 8)?
            .try_into()
            .map_err(|_| ManagedError::Corrupt)?,
    ))
}
