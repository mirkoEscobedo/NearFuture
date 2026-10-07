use super::*;
use crate::{
    PeerError,
    receipt::{OriginalReceipt, SourceMinima},
};
/// Trusted external recovery input; storing it beside the book does not protect rollback.
#[derive(Clone, Debug)]
pub struct BookAnchor {
    pub slot: u8,
    pub generation: u8,
    pub digest: [u8; 32],
    pub original: OriginalReceipt,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
    pub minimum: SourceMinima,
}
impl BookAnchor {
    pub fn from_record(slot: u8, record: &BookRecord) -> Result<Self, PeerError> {
        if slot >= 8 {
            return Err(PeerError::Limit);
        }
        Ok(Self {
            slot,
            generation: record.generation,
            digest: record.digest()?,
            original: record.original.clone(),
            ruleset: record.ruleset,
            content: record.content,
            minimum: record.minimum,
        })
    }
    pub(super) fn matches(&self, r: &BookRecord) -> bool {
        let p = self.original.source();
        let q = r.original.source();
        self.original.original() == r.original.original()
            && self.original.operation() == r.original.operation()
            && p.peer == q.peer
            && p.account == q.account
            && p.device == q.device
            && self.ruleset == r.ruleset
            && self.content == r.content
    }
}
/// Accepted observational heads, never active session/proof or permission state.
#[derive(Clone, Debug)]
pub struct BookCatalog {
    pub(super) heads: [Option<BookRecord>; BOOK_SLOTS],
}
impl BookCatalog {
    pub fn head(&self, slot: u8) -> Result<&BookRecord, PeerError> {
        self.heads
            .get(slot as usize)
            .and_then(Option::as_ref)
            .ok_or(PeerError::Storage)
    }
    pub fn anchors(&self) -> Result<Vec<BookAnchor>, PeerError> {
        self.heads
            .iter()
            .enumerate()
            .filter_map(|(slot, head)| {
                head.as_ref()
                    .map(|r| BookAnchor::from_record(slot as u8, r))
            })
            .collect()
    }
    pub fn occupied(&self) -> usize {
        self.heads.iter().filter(|v| v.is_some()).count()
    }
}
