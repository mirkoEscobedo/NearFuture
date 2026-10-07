use crate::{
    PeerError,
    receipt::{OriginalReceipt, ReceiptPhase, SourceMinima},
};
use sha2::{Digest, Sha256};
pub const BOOK_BYTES: usize = 479;
pub const BOOK_SLOTS: usize = 8;
pub const BOOK_GENERATIONS: usize = 8;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookPhase {
    Unobserved,
    Receipt(ReceiptPhase),
}
#[derive(Clone, Debug)]
pub struct BookRecord {
    pub generation: u8,
    pub previous: [u8; 32],
    pub original: OriginalReceipt,
    pub ruleset: [u8; 32],
    pub content: [u8; 32],
    pub minimum: SourceMinima,
    pub phase: BookPhase,
}
impl BookRecord {
    pub fn digest(&self) -> Result<[u8; 32], PeerError> {
        Ok(Sha256::digest(super::encode_book(self)?).into())
    }
    pub fn same_original(&self, other: &Self) -> bool {
        let a = self.original.source();
        let b = other.original.source();
        self.original.original() == other.original.original()
            && self.original.operation() == other.original.operation()
            && a.account == b.account
            && a.device == b.device
            && a.peer == b.peer
            && self.ruleset == other.ruleset
            && self.content == other.content
    }
    pub fn validate(&self) -> Result<(), PeerError> {
        if self.generation >= 8 || (self.generation == 0 && self.previous != [0; 32]) {
            return Err(PeerError::Malformed);
        }
        if let BookPhase::Receipt(
            ReceiptPhase::Rejected { sequence } | ReceiptPhase::Committed { sequence },
        ) = self.phase
            && (sequence.0 == 0 || sequence > self.minimum.event)
        {
            return Err(PeerError::Malformed);
        }
        Ok(())
    }
    pub fn follows(&self, old: &Self) -> Result<(), PeerError> {
        if self.generation != old.generation + 1
            || self.previous != old.digest()?
            || !self.same_original(old)
            || !self.minimum.admits(old.minimum)
        {
            return Err(PeerError::Replay);
        }
        match (old.phase, self.phase) {
            (BookPhase::Unobserved, _)
            | (BookPhase::Receipt(ReceiptPhase::Unknown), BookPhase::Receipt(_)) => {}
            (
                BookPhase::Receipt(ReceiptPhase::Pending),
                BookPhase::Receipt(
                    ReceiptPhase::Pending
                    | ReceiptPhase::Rejected { .. }
                    | ReceiptPhase::Committed { .. },
                ),
            ) => {}
            (BookPhase::Receipt(a), BookPhase::Receipt(b)) if a == b => {}
            _ => return Err(PeerError::Replay),
        }
        self.validate()
    }
}
