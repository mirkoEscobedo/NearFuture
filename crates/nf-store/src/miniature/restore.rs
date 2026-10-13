//! Owned isolated copying only; no archive can grant active history authority.
use crate::error::{Result, StoreError};
use rusqlite::{
    Connection,
    backup::{Backup, StepResult},
};
use std::{
    fs::{self, OpenOptions},
    path::Path,
};

/// Acknowledges only creating a new read-only inspection copy.
/// This is neither authentication nor permission to adopt or overwrite a history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IsolatedRestoreConfirmation {
    NewReadOnlyCopy,
}

pub(super) fn copy_to_new_path(
    source: &Connection,
    destination: &Path,
    confirmation: IsolatedRestoreConfirmation,
) -> Result<()> {
    let IsolatedRestoreConfirmation::NewReadOnlyCopy = confirmation;
    let page_size: u32 = source.pragma_query_value(None, "page_size", |row| row.get(0))?;
    let pages: u32 = source.pragma_query_value(None, "page_count", |row| row.get(0))?;
    if page_size != 4096 || pages == 0 || pages > 65536 {
        return Err(StoreError::Limit);
    }
    // The caller owns quiescent ancestors; reparse paths are not accepted.
    existing_parent(destination)?;
    absent(destination)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut companion = destination.as_os_str().to_os_string();
        companion.push(suffix);
        absent(Path::new(&companion))?;
    }
    // CreateNew also refuses the source path, aliases, or any other occupied file.
    crate::schema::reserve(destination)?;
    let mut connection = crate::schema::connection(destination)?;
    crate::schema::configure(&connection)?;
    {
        let backup = Backup::new(source, &mut connection)?;
        let mut complete = false;
        // A quiescent source cannot grow or restart. Bound the whole page loop.
        for _ in 0..pages.div_ceil(100) {
            let step = backup.step(100)?;
            match step {
                StepResult::Busy | StepResult::Locked => return Err(StoreError::Busy),
                StepResult::Done | StepResult::More => {}
                _ => return Err(StoreError::Io),
            }
            let progress = backup.progress();
            if progress.pagecount != pages as i32 || progress.remaining < 0 {
                return Err(StoreError::Limit);
            }
            if step == StepResult::Done {
                complete = true;
                break;
            }
        }
        if !complete {
            return Err(StoreError::Limit);
        }
    }
    connection
        .close()
        .map_err(|(_, error)| StoreError::from(error))?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(destination)
        .map_err(|_| StoreError::Io)?
        .sync_all()
        .map_err(|_| StoreError::Io)?;
    let length = fs::metadata(destination).map_err(|_| StoreError::Io)?.len();
    if !(100..=268_435_456).contains(&length) {
        return Err(StoreError::Limit);
    }
    Ok(())
}

fn absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(StoreError::AlreadyExists),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StoreError::Io),
    }
}

fn existing_parent(destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    for ancestor in parent.ancestors().filter(|path| !path.as_os_str().is_empty()) {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| StoreError::Io)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(StoreError::Io);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(StoreError::Io);
            }
        }
    }
    Ok(())
}
