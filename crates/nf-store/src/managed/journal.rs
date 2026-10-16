//! Narrow rollback companion checks. No deletion, journal-mode conversion or foreign recovery.
use super::{
    error::{ManagedError, Result},
    schema, world_file,
};
use rusqlite::Connection;
use std::{
    fs::{File, Metadata},
    io::Read,
    path::Path,
};

#[derive(Clone, Copy)]
pub(super) enum Admission {
    Absent,
    OwnedCold,
    OwnedRecovery,
}
pub(super) fn admit(path: &Path, admission: Admission) -> Result<()> {
    match admission {
        Admission::Absent => schema::companions_absent(path, false),
        Admission::OwnedCold => companion(path, false),
        Admission::OwnedRecovery => companion(path, true),
    }
}
fn stable(a: &Metadata, b: &Metadata) -> Result<()> {
    schema::regular_file(b)?;
    if a.len() != b.len() || a.modified()? != b.modified()? {
        return Err(ManagedError::SourceMismatch);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if a.dev() != b.dev() || a.ino() != b.ino() {
            return Err(ManagedError::SourceMismatch);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if a.creation_time() != b.creation_time() || a.file_attributes() != b.file_attributes() {
            return Err(ManagedError::SourceMismatch);
        }
    }
    Ok(())
}
/// Called only for the exact already-owned catalog path or catalog-bound pending artifact.
/// SQLite alone performs any hot recovery; this does not authorize arbitrary sidecars.
pub(super) fn companion(path: &Path, hot_recovery: bool) -> Result<()> {
    schema::absent(&schema::companion(path, "-wal"))?;
    schema::absent(&schema::companion(path, "-shm"))?;
    let journal = schema::companion(path, "-journal");
    let before = match std::fs::symlink_metadata(&journal) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(value) => value,
    };
    schema::regular_file(&before)?;
    if before.len() > 536_870_912 {
        return Err(ManagedError::Limit);
    }
    let mut file = File::open(&journal)?;
    stable(&before, &file.metadata()?)?;
    if before.len() != 0 {
        if before.len() < 28 {
            return Err(ManagedError::UnsupportedFormat);
        }
        let mut header = [0; 28];
        file.read_exact(&mut header)?;
        if header != [0; 28] {
            let pages = u32::from_be_bytes(
                header[8..12]
                    .try_into()
                    .map_err(|_| ManagedError::Corrupt)?,
            );
            let original = u32::from_be_bytes(
                header[16..20]
                    .try_into()
                    .map_err(|_| ManagedError::Corrupt)?,
            );
            let sector = u32::from_be_bytes(
                header[20..24]
                    .try_into()
                    .map_err(|_| ManagedError::Corrupt)?,
            );
            let page_size = u32::from_be_bytes(
                header[24..28]
                    .try_into()
                    .map_err(|_| ManagedError::Corrupt)?,
            );
            if !hot_recovery
                || header[..8] != [0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7]
                || (pages != u32::MAX && pages > 65_536)
                || original > 65_536
                || !(512..=65_536).contains(&sector)
                || !sector.is_power_of_two()
                || page_size != 4096
            {
                return Err(ManagedError::UnsupportedFormat);
            }
        }
    }
    stable(&before, &file.metadata()?)?;
    stable(&before, &std::fs::symlink_metadata(&journal)?)?;
    Ok(())
}
/// The retained real connection already acquired and keeps SQLite's exclusive gate.
/// A completed DELETE/EXCLUSIVE transaction may retain a zeroed or truncated journal.
pub(super) fn cold(connection: &Connection, path: &Path) -> Result<()> {
    world_file::bind_path(connection, path)?;
    if !connection.is_autocommit() {
        return Err(ManagedError::Quarantined);
    }
    let mode: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    let locking: String = connection.pragma_query_value(None, "locking_mode", |row| row.get(0))?;
    let sync: u32 = connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    let size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    if mode != "delete" || locking != "exclusive" || sync != 3 || size != 4096 {
        return Err(ManagedError::UnsupportedFormat);
    }
    companion(path, false)
}
