//! Conservative prewrite room for the exact bundled SQLite3.46.0 active quota operation.
//! See the companion primary-source allocation derivation; this is an upper bound, not a prediction.
use super::{error::{ManagedError, Result}, rows::{Cell, Row}, world_plan::Plan};
use rusqlite::Connection;

const SOURCE_ID: &str = "2024-05-23 13:25:27 96c92aba00c8375bc32fafcdf12429c58bd8aabfcadab6683e35bbb9cdebf19e";
const MIN_OVERFLOW_BYTES: u64 = 4096 - 255 - 4;
// At most21 visited balance levels including a new root child, <=NB+2=5 allocations/level,
// plus one root-copy allocation; double each allocation for a possible pointer-map page.
const BALANCE_ROOM: u64 = 2 * (21 * 5 + 1);
fn payload(row: &[Cell]) -> Result<u64> {
    if row.len() > 64 { return Err(ManagedError::Limit); }
    // One at-most-nine-byte serial type per field, one header-length varint, exact BLOB
    // bytes and at most8 bytes per integer. Extra32 covers the outer cell framing.
    let mut size = 9_u64.checked_mul(row.len() as u64 + 1).and_then(|n| n.checked_add(32))
        .ok_or(ManagedError::Limit)?;
    for cell in row {
        let bytes = match cell { Cell::Null => 0, Cell::Integer(_) => 8,
            Cell::Blob(bytes) => bytes.len().try_into().map_err(|_| ManagedError::Limit)? };
        size = size.checked_add(bytes).ok_or(ManagedError::Limit)?;
    }
    Ok(size)
}
fn indices(name: &str) -> Result<&'static [&'static [usize]]> {
    match name {
        "active_meta" => Ok(&[]), "active_journal" | "active_snapshots" => Ok(&[&[0]]),
        "active_events" => Ok(&[&[0], &[1]]),
        _ => Err(ManagedError::OperationConflict),
    }
}
fn insert_room(row: &Row, indices: &[&[usize]]) -> Result<u64> {
    let mut room = BALANCE_ROOM.checked_add(2 * payload(row)?.div_ceil(MIN_OVERFLOW_BYTES))
        .ok_or(ManagedError::Limit)?;
    for fields in indices {
        let mut key = Vec::new(); let mut bytes = 0_usize;
        for field in *fields {
            let value = row.get(*field).ok_or(ManagedError::Corrupt)?;
            let Cell::Blob(blob) = value else { return Err(ManagedError::Corrupt); };
            bytes = bytes.checked_add(blob.len()).ok_or(ManagedError::Limit)?;
            key.push(value.clone());
        }
        if bytes > 32 { return Err(ManagedError::Limit); }
        key.push(Cell::Integer(i64::MAX));
        room = room.checked_add(BALANCE_ROOM)
            .and_then(|n| n.checked_add(2 * payload(&key).ok()?.div_ceil(MIN_OVERFLOW_BYTES)))
            .ok_or(ManagedError::Limit)?;
    }
    Ok(room)
}
pub(super) fn active_quota(c: &Connection, plan: &Plan, quota: u32) -> Result<()> {
    let source: String = c.query_row("SELECT sqlite_source_id()", [], |row| row.get(0))?;
    if source != SOURCE_ID { return Err(ManagedError::UnsupportedFormat); }
    let pages: u64 = c.pragma_query_value(None, "page_count", |row| row.get(0))?;
    let page_size: u32 = c.pragma_query_value(None, "page_size", |row| row.get(0))?;
    if page_size != 4096 || pages > 65_536 { return Err(ManagedError::Limit); }
    let mut added = 0_u64;
    for (name, patch) in &plan.patches {
        let indices = indices(name)?;
        for (before, after) in patch.0.values() {
            if before.is_some() {
                added = added.checked_add(BALANCE_ROOM * (1 + indices.len() as u64)).ok_or(ManagedError::Limit)?;
            }
            if let Some(row) = after { added = added.checked_add(insert_room(row, indices)?).ok_or(ManagedError::Limit)?; }
        }
    }
    // Ignore freelist/deletion credit. No table/trigger/DDL/extension beyond the strict selected schema.
    if pages.checked_add(added).ok_or(ManagedError::Limit)? > u64::from(quota) { return Err(ManagedError::Limit); }
    Ok(())
}
