use crate::receipt::{OriginalReceipt, ReceiptStatus, ReceiptUnsupportedReason, SourceMinima};
use std::time::Instant;
/// Opaque fresh proof result. Only repository persistence may expose a fresh status.
/// No Clone, saved session, strategic Ack conversion or public constructor exists.
pub(crate) struct VerifiedReceipt {
    pub(in crate::receipt_effects) context: crate::records::PeerContext,
    pub(in crate::receipt_effects) original: OriginalReceipt,
    pub(in crate::receipt_effects) result: VerifiedResult,
    pub(in crate::receipt_effects) minimum: SourceMinima,
    pub(in crate::receipt_effects) revision: u64,
    pub(in crate::receipt_effects) created: Instant,
    pub(in crate::receipt_effects) not_after: Option<Instant>,
}
pub(in crate::receipt_effects) enum VerifiedResult {
    Status(ReceiptStatus),
    Unsupported(ReceiptUnsupportedReason),
}
#[derive(Clone, Debug)]
pub enum AdmittedReceipt {
    Status(ReceiptStatus),
    Unsupported(ReceiptUnsupportedReason),
}

impl VerifiedReceipt {
    pub(in crate::receipt_effects) fn live(&self) -> Result<(), crate::PeerError> {
        fresh_deadline(self.not_after)
    }
}

pub(in crate::receipt_effects) fn fresh_deadline(
    end: Option<Instant>,
) -> Result<(), crate::PeerError> {
    if end.is_some_and(|end| Instant::now() >= end) {
        Err(crate::PeerError::Replay)
    } else {
        Ok(())
    }
}
