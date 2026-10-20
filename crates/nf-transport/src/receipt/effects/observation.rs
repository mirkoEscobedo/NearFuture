use super::{
    ReceiptRepo,
    book::*,
    operation::{AdmittedReceipt, VerifiedReceipt, VerifiedResult, fresh},
};
use crate::{
    PeerError,
    receipt::{OriginalReceipt, SourceMinima},
    records::PeerContext,
};
impl ReceiptRepo {
    pub(crate) fn require_context(&self, c: &PeerContext) -> Result<(), PeerError> {
        if c.scope != self.config.scope
            || c.ruleset != self.config.ruleset
            || c.content != self.config.content
        {
            return Err(PeerError::Scope);
        }
        Ok(())
    }
    pub(super) fn protected_minimum(
        &self,
        original: &OriginalReceipt,
    ) -> Result<SourceMinima, PeerError> {
        self.validate_book_context()?;
        let anchor = self
            .anchors
            .iter()
            .find(|a| a.original.request() == original.request())
            .ok_or(PeerError::Storage)?;
        if anchor.original.original() != original.original()
            || anchor.original.operation() != original.operation()
        {
            return Err(PeerError::Unauthorized);
        }
        Ok(anchor.minimum)
    }
    /// Recovery is observational. The caller obtains a fresh active session before querying.
    pub fn original_for_slot(
        &self,
        slot: u8,
    ) -> Result<(OriginalReceipt, SourceMinima), PeerError> {
        let catalog = self.recover_observations()?;
        let head = catalog.head(slot)?;
        Ok((head.original.clone(), head.minimum))
    }
    /// A fresh authenticated status becomes public only after immutable persistence succeeds.
    /// Expiry/policy failure after persistence returns no fresh result; saved data is observational.
    pub(crate) fn accept_verified(
        &mut self,
        slot: u8,
        verified: VerifiedReceipt,
    ) -> Result<AdmittedReceipt, PeerError> {
        self.require_context(&verified.context)?;
        self.validate_book_context()?;
        fresh(verified.created)?;
        verified.live()?;
        self.validate_original(&verified.original)?;
        let anchor = self
            .anchors
            .iter()
            .find(|anchor| anchor.slot == slot)
            .ok_or(PeerError::Storage)?;
        let a = anchor.original.source();
        let b = verified.original.source();
        if anchor.original.original() != verified.original.original()
            || anchor.original.operation() != verified.original.operation()
            || a.peer != b.peer
            || a.account != b.account
            || a.device != b.device
        {
            return Err(PeerError::Unauthorized);
        }
        self.with_current_read(|cut| {
            fresh(verified.created)?;
            verified.live()?;
            if cut.membership.revision != verified.revision {
                return Err(PeerError::Policy);
            }
            Ok(())
        })?;
        let not_after = verified.not_after;
        match verified.result {
            VerifiedResult::Unsupported(reason) => Ok(AdmittedReceipt::Unsupported(reason)),
            VerifiedResult::Status(status) => {
                let catalog =
                    append_receipt(&self.vault, &self.anchors, slot, &status, verified.minimum)?;
                let head = catalog.head(slot)?;
                if head.original.original() != verified.original.original()
                    || head.original.operation() != verified.original.operation()
                {
                    return Err(PeerError::Unauthorized);
                }
                // Keep the durable head after successful persistence even if the final policy cut fails.
                self.anchors = catalog.anchors()?;
                self.validate_book_context()?;
                fresh(verified.created)?;
                super::operation::fresh_deadline(not_after)?;
                self.with_current_read(|cut| {
                    fresh(verified.created)?;
                    super::operation::fresh_deadline(not_after)?;
                    if cut.membership.revision != verified.revision {
                        return Err(PeerError::Policy);
                    }
                    Ok(())
                })?;
                Ok(AdmittedReceipt::Status(status))
            }
        }
    }
}
