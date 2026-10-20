//! Local continuation provenance only. It is never a remote identity/session grant.
use super::{ReceiptRepo, operation::fresh};
use crate::{
    PeerError,
    receipt::{OriginalReceipt, SourceMinima},
    records::PeerContext,
};
use std::time::Instant;
pub(crate) struct ReceiptQueryBinding {
    pub(super) nonce: [u8; 32],
    pub(super) slot: u8,
    pub(super) original: OriginalReceipt,
    pub(super) context: PeerContext,
    pub(super) minimum: SourceMinima,
    pub(super) not_after: Instant,
}
impl ReceiptQueryBinding {
    /// Trusted Notify/Portal issuer supplies a fresh local correlation; this creates no admission.
    pub(crate) fn new(
        nonce: [u8; 32],
        slot: u8,
        original: OriginalReceipt,
        context: PeerContext,
        minimum: SourceMinima,
        not_after: Instant,
    ) -> Result<Self, PeerError> {
        if Instant::now() >= not_after {
            return Err(PeerError::Replay);
        }
        if nonce == [0; 32]
            || slot >= 8
            || context.session != [0; 16]
            || original.original().universe_id != context.scope.universe
            || original.original().history_id != context.scope.history
            || minimum.membership_revision < original.source().minimum_membership
        {
            return Err(PeerError::Session);
        }
        Ok(Self {
            nonce,
            slot,
            original,
            context,
            minimum,
            not_after,
        })
    }
    pub(super) fn live(&self) -> Result<(), PeerError> {
        if Instant::now() >= self.not_after {
            Err(PeerError::Replay)
        } else {
            Ok(())
        }
    }
    pub(crate) fn correlation(&self) -> [u8; 32] {
        self.nonce
    }
}
/// Issued only by an actual Lane Status admission after immutable Repo persistence succeeds.
/// No Clone, public constructor, or saved-grant representation exists.
pub struct ReceiptCompletion {
    pub(super) correlation: Option<[u8; 32]>,
    pub(super) slot: u8,
    pub(super) original: OriginalReceipt,
    pub(super) context: PeerContext,
    pub(super) current: SourceMinima,
    pub(super) revision: u64,
    pub(super) created: Instant,
}
impl ReceiptCompletion {
    /// Consumes one exact local query continuation after a new fresh repository cut.
    pub(crate) fn validate_query(
        self,
        nonce: [u8; 32],
        slot: u8,
        original: &OriginalReceipt,
        context: PeerContext,
        minimum: SourceMinima,
        repo: &mut ReceiptRepo,
    ) -> Result<SourceMinima, PeerError> {
        if self.correlation != Some(nonce)
            || self.slot != slot
            || !same_original(&self.original, original)
            || self.context.scope != context.scope
            || self.context.ruleset != context.ruleset
            || self.context.content != context.content
            || !self.current.admits(minimum)
        {
            return Err(PeerError::Unauthorized);
        }
        fresh(self.created)?;
        repo.require_context(&self.context)?;
        repo.validate_original(&self.original)?;
        if !self.current.admits(repo.protected_minimum(&self.original)?) {
            return Err(PeerError::Policy);
        }
        repo.with_current_read(|cut| {
            fresh(self.created)?;
            if cut.membership.revision != self.revision
                || self.current.membership_revision != self.revision
            {
                return Err(PeerError::Policy);
            }
            Ok(self.current)
        })
    }
}
pub(super) fn same_original(a: &OriginalReceipt, b: &OriginalReceipt) -> bool {
    let x = a.source();
    let y = b.source();
    a.original() == b.original()
        && a.operation() == b.operation()
        && x.peer == y.peer
        && x.account == y.account
        && x.device == y.device
        && x.minimum_membership == y.minimum_membership
}

impl std::fmt::Debug for ReceiptCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReceiptCompletion(REDACTED)")
    }
}
