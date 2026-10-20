use crate::error::{Result, StoreError};
use rusqlite::{Connection, OpenFlags, limits::Limit};
use std::{path::Path, time::Duration};

/// Opens only a quiescent rollback-journal file, without recovery or write access.
pub(crate) fn read_only_connection(path: &Path) -> Result<Connection> {
    use std::io::Read;
    // Mapped-drive classification remains the host's local-storage precondition.
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next()
        && !matches!(
            prefix.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
        )
    {
        return Err(StoreError::Io);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            StoreError::MissingHistory
        } else {
            StoreError::Io
        }
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(StoreError::Io);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(StoreError::Io);
        }
    }
    if metadata.len() < 100 || metadata.len() > 268_435_456 {
        return Err(StoreError::MissingHistory);
    }
    let mut header = [0; 20];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|_| StoreError::Io)?;
    if &header[..16] != b"SQLite format 3\0" {
        return Err(StoreError::Corrupt);
    }
    if header[18] != 1 || header[19] != 1 {
        return Err(StoreError::UnsupportedSchema);
    }
    for (suffix, refusal) in [
        ("-wal", StoreError::UnsupportedSchema),
        ("-shm", StoreError::UnsupportedSchema),
        ("-journal", StoreError::Busy),
    ] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        match std::fs::symlink_metadata(Path::new(&sidecar)) {
            Ok(_) => return Err(refusal),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(StoreError::Io),
        }
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(Duration::from_millis(250))?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 2_097_152);
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 65_536);
    connection.set_limit(Limit::SQLITE_LIMIT_COLUMN, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 128);
    connection.set_limit(Limit::SQLITE_LIMIT_EXPR_DEPTH, 64);
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    connection.execute_batch(
        "PRAGMA query_only=ON; PRAGMA trusted_schema=OFF; PRAGMA cache_size=-2048;",
    )?;
    let query_only: i32 = connection.pragma_query_value(None, "query_only", |row| row.get(0))?;
    let trusted: i32 = connection.pragma_query_value(None, "trusted_schema", |row| row.get(0))?;
    if !connection.is_readonly(rusqlite::DatabaseName::Main)? || query_only != 1 || trusted != 0 {
        return Err(StoreError::Io);
    }
    Ok(connection)
}
