use super::*;
use crate::{
    receipt_effects::{ReceiptCompletion, ReceiptLane, ReceiptQueryBinding},
    records::PeerContext,
};
pub(super) struct Query {
    nonce: [u8; 32],
    generation: u64,
    slot: u8,
    original: OriginalReceipt,
    context: PeerContext,
    minimum: SourceMinima,
}
fn same_original(a: &OriginalReceipt, b: &OriginalReceipt) -> bool {
    let x = a.source();
    let y = b.source();
    a.original() == b.original()
        && a.operation() == b.operation()
        && x.peer == y.peer
        && x.account == y.account
        && x.device == y.device
        && x.minimum_membership == y.minimum_membership
}
impl NotifyLane {
    #[cfg(test)]
    pub(in crate::notification_effects) fn remaining_for_test(
        &self,
    ) -> Result<Duration, PeerError> {
        match &self.endpoint {
            Endpoint::Client {
                subscriber: Some(s),
                ..
            } => s.remaining_for_test(),
            _ => Err(PeerError::Session),
        }
    }
    /// Starts one fresh control2 operation for the privately retained dirty generation.
    pub fn request_followup(
        &mut self,
        receipts: &mut ReceiptLane,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.guard(repo)?;
        if self.query.is_some() {
            return Err(PeerError::Backpressure);
        }
        let Endpoint::Client {
            subscriber: Some(s),
            original,
            slot,
            ..
        } = &self.endpoint
        else {
            return Err(PeerError::Session);
        };
        s.active_live()?;
        let generation = s.followup_generation()?;
        let (retained, mut minimum) = repo.original_for_slot(*slot)?;
        if !same_original(original, &retained) {
            return Err(PeerError::Unauthorized);
        }
        minimum.membership_revision = minimum.membership_revision.max(s.session.revision);
        let context = PeerContext {
            session: [0; 16],
            scope: self.binding.scope,
            ruleset: self.binding.ruleset,
            content: self.binding.content,
        };
        let nonce = crate::session::fresh()?;
        let binding = ReceiptQueryBinding::new(
            nonce,
            *slot,
            original.clone(),
            context,
            minimum,
            s.not_after()?,
        )?;
        let query = Query {
            nonce: binding.correlation(),
            generation,
            slot: *slot,
            original: original.clone(),
            context,
            minimum,
        };
        s.active_live()?;
        receipts.request_for_query(repo, binding)?;
        s.active_live()?;
        self.query = Some(query);
        Ok(())
    }
    /// Only a non-Clone certificate minted by actual persisted control2 Status admission
    /// can cover the exact query-start nonce and generation. Public outcome data cannot.
    pub fn accept_receipt_completion(
        &mut self,
        completion: ReceiptCompletion,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.accept_completion_core(completion, repo, || {})
    }
    #[cfg(test)]
    pub(in crate::notification_effects) fn accept_completion_after_validation(
        &mut self,
        completion: ReceiptCompletion,
        repo: &mut ReceiptRepo,
        after_validation: impl FnOnce(),
    ) -> Result<(), PeerError> {
        self.accept_completion_core(completion, repo, after_validation)
    }
    fn accept_completion_core(
        &mut self,
        completion: ReceiptCompletion,
        repo: &mut ReceiptRepo,
        after_validation: impl FnOnce(),
    ) -> Result<(), PeerError> {
        let query = self.query.take().ok_or(PeerError::Replay)?;
        self.guard(repo)?;
        let Endpoint::Client {
            subscriber: Some(s),
            ..
        } = &mut self.endpoint
        else {
            return Err(PeerError::Session);
        };
        s.active_live()?;
        completion.validate_query(
            query.nonce,
            query.slot,
            &query.original,
            query.context,
            query.minimum,
            repo,
        )?;
        after_validation();
        s.active_live()?;
        s.cover_generation(query.generation)
    }
    /// Failed, unsupported or lost queries release only the in-flight marker, never dirty state.
    pub fn receipt_failure(&mut self) {
        self.query = None;
    }
}
