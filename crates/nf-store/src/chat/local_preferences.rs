//! Initial immutable local preference snapshot; no ChatStore or membership authority.
use super::{ChatStoreError, Result};
use nf_contract::identity::AccountId;
use nf_identity::model::Scope;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const MAGIC: &[u8; 8] = b"NF-MUTE1";
const HEADER_BYTES: usize = 57;
const MAX_LOCAL_MUTED_ACCOUNTS: usize = 64;
const MAX_FILE_BYTES: usize = HEADER_BYTES + MAX_LOCAL_MUTED_ACCOUNTS * 16;

fn validate_viewer(viewer: AccountId) -> Result<()> {
    if viewer.as_bytes() == &[0; 16] {
        return Err(ChatStoreError::Malformed);
    }
    Ok(())
}

/// Saves one initial snapshot into an explicitly caller-owned, absent file.
/// Existing files are refused. Write/sync failures leave the partial original for inspection.
/// This is not replacement, directory-fsync, crash-safe update, or game-save authority.
pub fn save_new_local_mutes(
    path: &Path,
    scope: Scope,
    viewer: AccountId,
    muted_accounts: &[AccountId],
) -> Result<()> {
    validate_viewer(viewer)?;
    if muted_accounts.len() > MAX_LOCAL_MUTED_ACCOUNTS {
        return Err(ChatStoreError::Limit);
    }
    if muted_accounts
        .iter()
        .any(|account| account.as_bytes() == &[0; 16])
    {
        return Err(ChatStoreError::Malformed);
    }
    let mut bytes = Vec::with_capacity(HEADER_BYTES + muted_accounts.len() * 16);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(scope.universe.as_bytes());
    bytes.extend_from_slice(scope.history.as_bytes());
    bytes.extend_from_slice(viewer.as_bytes());
    bytes.push(muted_accounts.len() as u8);
    for account in muted_accounts {
        bytes.extend_from_slice(account.as_bytes());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                ChatStoreError::AlreadyExists
            } else {
                ChatStoreError::Storage
            }
        })?;
    file.write_all(&bytes)
        .map_err(|_| ChatStoreError::Storage)?;
    file.sync_all().map_err(|_| ChatStoreError::Storage)
}

/// Reads at most the fixed maximum plus one byte to refuse oversized files.
/// Missing, malformed or wrong-scope/viewer snapshots fail explicitly, without a mute reset.
/// The local file is display input, not authenticated identity or network policy.
pub fn load_local_mutes(path: &Path, scope: Scope, viewer: AccountId) -> Result<Vec<AccountId>> {
    validate_viewer(viewer)?;
    let file = File::open(path).map_err(|_| ChatStoreError::Storage)?;
    let mut bytes = Vec::new();
    file.take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ChatStoreError::Storage)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(ChatStoreError::Limit);
    }
    if bytes.len() < HEADER_BYTES || &bytes[..8] != MAGIC {
        return Err(ChatStoreError::Corrupt);
    }
    if &bytes[8..24] != scope.universe.as_bytes()
        || &bytes[24..40] != scope.history.as_bytes()
        || &bytes[40..56] != viewer.as_bytes()
    {
        return Err(ChatStoreError::Scope);
    }
    let count = bytes[56] as usize;
    if count > MAX_LOCAL_MUTED_ACCOUNTS {
        return Err(ChatStoreError::Limit);
    }
    if bytes.len() != HEADER_BYTES + count * 16 {
        return Err(ChatStoreError::Corrupt);
    }
    bytes[HEADER_BYTES..]
        .as_chunks::<16>()
        .0
        .iter()
        .map(|chunk| {
            let account = AccountId::from_slice(chunk).map_err(|_| ChatStoreError::Corrupt)?;
            if account.as_bytes() == &[0; 16] {
                return Err(ChatStoreError::Corrupt);
            }
            Ok(account)
        })
        .collect()
}
