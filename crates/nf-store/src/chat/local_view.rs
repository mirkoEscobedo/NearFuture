//! Local display selection only. This module grants no membership or signature authority.
use super::{ChatStoreError, HistoryPage, MAX_TEXT_BYTES, Result};
use nf_contract::identity::AccountId;
use nf_identity::model::Scope;

// A local preference snapshot cap, independently chosen for this projection.
// It is not a membership, transport, or archive capacity claim.
const MAX_LOCAL_MUTED_ACCOUNTS: usize = 64;
const MAX_SOURCE_PAGE_ENTRIES: usize = 64;

/// Filters an already admitted source page for a local viewer.
/// The caller supplies the page from the unchanged authenticated history boundary.
/// This does not block network delivery, erase durable originals, or confer moderator authority.
pub fn project_local_mutes(
    scope: Scope,
    muted_accounts: &[AccountId],
    source: &HistoryPage,
) -> Result<HistoryPage> {
    if muted_accounts.len() > MAX_LOCAL_MUTED_ACCOUNTS
        || source.entries.len() > MAX_SOURCE_PAGE_ENTRIES
    {
        return Err(ChatStoreError::Limit);
    }
    if muted_accounts
        .iter()
        .any(|account| account.as_bytes() == &[0; 16])
    {
        return Err(ChatStoreError::Malformed);
    }
    for entry in &source.entries {
        if entry.signed.message.scope != scope {
            return Err(ChatStoreError::Scope);
        }
        if entry.signed.message.text.len() > MAX_TEXT_BYTES {
            return Err(ChatStoreError::Limit);
        }
    }
    let entries = source
        .entries
        .iter()
        .filter(|entry| !muted_accounts.contains(&entry.signed.message.author.account))
        .cloned()
        .collect();
    Ok(HistoryPage {
        entries,
        // This source cursor advances even when every display entry is muted.
        // It never becomes a global or federated message order.
        next_cursor: source.next_cursor,
    })
}
