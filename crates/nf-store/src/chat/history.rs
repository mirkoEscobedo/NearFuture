use super::{HistoryEntry, HistoryPage, HistoryQuery, Result, ledger::State};
// Invoked only after the same transaction's fresh current reader proof has been consumed.
pub(super) fn read(state: &State, query: &HistoryQuery) -> Result<HistoryPage> {
    let mut candidates: Vec<_> = state
        .messages
        .values()
        .filter(|entry| entry.receipt.receiver_cursor > query.after_cursor)
        .collect();
    candidates.sort_by_key(|entry| entry.receipt.receiver_cursor);
    let entries: Vec<_> = candidates
        .into_iter()
        .take(usize::from(query.limit))
        .map(|entry| HistoryEntry {
            receiver_cursor: entry.receipt.receiver_cursor,
            signed: entry.signed.clone(),
        })
        .collect();
    let next_cursor = entries
        .last()
        .map_or(query.after_cursor, |entry| entry.receiver_cursor);
    Ok(HistoryPage {
        entries,
        next_cursor,
    })
}
