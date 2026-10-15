use super::{error::{ManagedError, Result}, input::FilePin, schema::{self, Profile}};
use rusqlite::{Connection, OpenFlags, backup::{Backup, StepResult}};
use sha2::{Digest, Sha256};
use std::{fs::{File, Metadata}, io::{Read, Write}, path::Path};

fn ordinary(meta: &Metadata) -> Result<()> {
    if !meta.is_file() || meta.file_type().is_symlink() { return Err(ManagedError::SourceMismatch); }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 { return Err(ManagedError::SourceMismatch); }
    }
    Ok(())
}
fn stable(before: &Metadata, after: &Metadata) -> Result<()> {
    ordinary(after)?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(ManagedError::SourceMismatch);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() { return Err(ManagedError::SourceMismatch); }
    }
    Ok(())
}
pub(super) fn observe(path: &Path, provenance: &str) -> Result<FilePin> {
    schema::local_absolute(path)?;
    let before = std::fs::symlink_metadata(path)?; ordinary(&before)?;
    if before.len() == 0 || before.len() > 268_435_456 { return Err(ManagedError::Limit); }
    let mut file = File::open(path)?; let opened = file.metadata()?; stable(&before, &opened)?;
    let mut digest = Sha256::new(); let mut left = before.len(); let mut buffer = [0_u8; 4096];
    let mut calls = 0_u32;
    while left != 0 {
        calls = calls.checked_add(1).ok_or(ManagedError::Limit)?;
        if calls > 65_536 { return Err(ManagedError::Limit); }
        let count = usize::try_from(left.min(4096)).map_err(|_| ManagedError::Limit)?;
        if file.read(&mut buffer[..count])? != count { return Err(ManagedError::SourceMismatch); }
        digest.update(&buffer[..count]); left -= u64::try_from(count).map_err(|_| ManagedError::Limit)?;
    }
    if file.read(&mut buffer[..1])? != 0 { return Err(ManagedError::SourceMismatch); }
    stable(&opened, &file.metadata()?)?; stable(&opened, &std::fs::symlink_metadata(path)?)?;
    FilePin::new(path.to_path_buf(), opened.len(), digest.finalize().into(), provenance)
}
pub(super) fn verify(pin: &FilePin) -> Result<()> {
    let actual = observe(&pin.path, &pin.provenance)?;
    if actual.bytes != pin.bytes || actual.digest != pin.digest { return Err(ManagedError::SourceMismatch); }
    Ok(())
}
pub(super) fn retained_copy(connection: &Connection, source: &Path, destination: &Path, provenance: &str) -> Result<FilePin> {
    schema::local_absolute(source)?; schema::local_absolute(destination)?;
    if source == destination { return Err(ManagedError::SourceMismatch); }
    super::journal::cold(connection, source)?;
    let pin = observe(source, provenance)?;
    let mut from = File::open(source)?; let before = from.metadata()?; ordinary(&before)?;
    if before.len() != pin.bytes { return Err(ManagedError::SourceMismatch); }
    let mut to = schema::reserve(destination)?;
    let mut remaining = pin.bytes; let mut buffer = [0_u8; 4096]; let mut digest = Sha256::new();
    let mut calls = 0_u32;
    while remaining != 0 {
        calls = calls.checked_add(1).ok_or(ManagedError::Limit)?;
        if calls > 65_536 { return Err(ManagedError::Limit); }
        let count = usize::try_from(remaining.min(4096)).map_err(|_| ManagedError::Limit)?;
        if from.read(&mut buffer[..count])? != count { return Err(ManagedError::SourceMismatch); }
        to.write_all(&buffer[..count])?; digest.update(&buffer[..count]);
        remaining -= u64::try_from(count).map_err(|_| ManagedError::Limit)?;
    }
    if from.read(&mut buffer[..1])? != 0 { return Err(ManagedError::SourceMismatch); }
    let copied: [u8; 32] = digest.finalize().into();
    if copied != pin.digest { return Err(ManagedError::SourceMismatch); }
    stable(&before, &from.metadata()?)?; stable(&before, &std::fs::symlink_metadata(source)?)?;
    to.sync_all()?; drop(to); drop(from);
    let result = observe(destination, provenance)?;
    if (result.bytes, result.digest) != (pin.bytes, pin.digest) { return Err(ManagedError::TargetMismatch); }
    super::journal::cold(connection, source)?; verify(&pin)?; Ok(result)
}
pub(super) fn backup(source: &Connection, destination: &Path, profile: Profile, quota: u32,
    expected_pages: u32, max_steps: u32) -> Result<Connection> {
    let pages: u32 = source.pragma_query_value(None, "page_count", |r| r.get(0))?;
    if pages == 0 || pages != expected_pages || pages > quota || max_steps != pages.div_ceil(100)
        || max_steps > 656 { return Err(ManagedError::Limit); }
    let reservation = schema::reserve(destination)?; drop(reservation);
    let mut target = Connection::open_with_flags(destination,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    schema::limits(&target, profile)?;
    target.pragma_update(None, "page_size", 4096)?;
    schema::configure(&target, profile, quota)?;
    {
        let backup = Backup::new(source, &mut target)?; let mut done = false;
        for _ in 0..max_steps {
            match backup.step(100)? {
                StepResult::Done => { done = true; break; },
                StepResult::More => {},
                StepResult::Busy | StepResult::Locked => return Err(ManagedError::Busy),
                _ => return Err(ManagedError::UnsupportedFormat),
            }
        }
        if !done { return Err(ManagedError::Limit); }
    }
    schema::configure(&target, profile, quota)?;
    let actual: u32 = target.pragma_query_value(None, "page_count", |r| r.get(0))?;
    if actual > quota { return Err(ManagedError::Limit); }
    Ok(target)
}
