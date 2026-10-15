use super::{error::{ManagedError, Result}, pending::Pending, schema,
    world_file::{self, WorldFile}};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

/// One owned pending-world open. Exact durable rows select pre/post before its admitted quota is applied.
pub(super) fn open(path: PathBuf, pending: &Pending) -> Result<(WorldFile, bool)> {
    schema::local_absolute(&path)?; schema::companions_absent(&path, true)?;
    let metadata = std::fs::symlink_metadata(&path)?;
    schema::regular_file(&metadata)?;
    if metadata.len() > 268_435_456 {
        return Err(ManagedError::UnsupportedFormat);
    }
    pending.validate()?;
    schema::header_selected(&path, &[world_file::profile(pending.pre.profile)?, world_file::profile(pending.post.profile)?])?;
    let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    schema::limits(&connection, world_file::profile(pending.pre.profile)?)?;
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version != pending.pre.profile && version != pending.post.profile { return Err(ManagedError::UnsupportedFormat); }
    let profile = world_file::profile(version)?; schema::physical(&connection, profile)?;
    let quota = if version == 4 {
        let bytes: Vec<u8> = connection.query_row("SELECT quota FROM active_meta WHERE singleton=1", [], |row| row.get(0))?;
        let value = u32::from_le_bytes(bytes.as_slice().try_into().map_err(|_| ManagedError::Corrupt)?);
        if value != pending.pre.quota && value != pending.post.quota { return Err(ManagedError::Limit); }
        value
    } else { pending.pre.quota };
    schema::configure(&connection, profile, quota)?; world_file::bind_path(&connection, &path)?;
    let pre = if version == pending.pre.profile && quota == pending.pre.quota {
        match world_file::inspect(&connection, &pending.pre, None) {
            Ok(_) => true, Err(ManagedError::Busy) => return Err(ManagedError::Busy), Err(_) => false,
        }
    } else { false };
    let post = if !pre && version == pending.post.profile && quota == pending.post.quota
        && !(pending.pre.profile == 1 && pending.purpose == 6) {
        match world_file::inspect(&connection, &pending.post, pending.intended.retirement) {
            Ok(_) => true, Err(ManagedError::Busy) => return Err(ManagedError::Busy), Err(_) => false,
        }
    } else { false };
    if !pre && !post { return Err(ManagedError::ReconciliationRequired); }
    let foot = if post { pending.post.clone() } else { pending.pre.clone() };
    let state = world_file::inspect(&connection, &foot, if post { pending.intended.retirement } else { None })?;
    let core_bytes = crate::codec::encode(&state.core)?; let envelope_bytes = state.encode()?;
    Ok((WorldFile { connection, path, state, foot, core_bytes, envelope_bytes, quarantined: false }, post))
}
