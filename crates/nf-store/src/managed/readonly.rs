//! Data-only admission of a closed inactive/retained file; never an owner or quota grant.
use super::{
    active_read,
    error::{ManagedError, Result},
    foot::StorageFoot,
    retired_read, schema, source_plan,
    state::FullState,
    world_file,
};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

pub(super) fn verify(
    path: &Path,
    expected: &StorageFoot,
    retirement: Option<[u8; 32]>,
) -> Result<FullState> {
    schema::local_absolute(path)?;
    schema::companions_absent(path, false)?;
    let profile = world_file::profile(expected.profile)?;
    schema::header(path, profile)?;
    let metadata = std::fs::symlink_metadata(path)?;
    schema::regular_file(&metadata)?;
    if metadata.len() > u64::from(expected.quota) * 4096 {
        return Err(ManagedError::Limit);
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    schema::limits(&connection, profile)?;
    connection.execute_batch(
        "PRAGMA trusted_schema=OFF; PRAGMA query_only=ON; PRAGMA cache_size=-2048;",
    )?;
    let query_only: u32 = connection.pragma_query_value(None, "query_only", |row| row.get(0))?;
    let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    let size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    let pages: u32 = connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
    if query_only != 1
        || journal != "delete"
        || size != 4096
        || pages == 0
        || pages > expected.quota
    {
        return Err(ManagedError::UnsupportedFormat);
    }
    schema::physical(&connection, profile)?;
    world_file::bind_path(&connection, path)?;
    // A read-only connection cannot grant a durable quota. The exact protected Foot/active
    // envelope supplies Q, while actual pages must fit it; no default65536 restoration occurs.
    let state = match expected.profile {
        1 => {
            source_plan::inspect_connection(
                &connection,
                expected.known,
                expected.quota,
                expected.policy,
                Some(expected),
            )?
            .0
        }
        4 => active_read::inspect(&connection, expected.known, Some(expected))?.state,
        5 => retired_read::inspect(&connection, expected, retirement)?.0,
        _ => return Err(ManagedError::UnsupportedFormat),
    };
    connection
        .close()
        .map_err(|(_, error)| ManagedError::from(error))?;
    schema::companions_absent(path, false)?;
    Ok(state)
}
