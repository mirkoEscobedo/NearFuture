use crate::{Driver, DriverError};
use nf_store::{
    StoreError,
    miniature::{ChallengeRequest, MiniatureCancelCause},
};
impl Driver {
    /// Reauthorizes every original intent; the Store retains its immutable request binding.
    pub fn resume_pending(&mut self) -> Result<nf_store::Accepted, DriverError> {
        let intents = self
            .store
            .pending()
            .ok_or(StoreError::InvalidTransition)?
            .intents()
            .to_vec();
        for intent in &intents {
            if self.signer_for(intent.actor, intent.device).is_none() {
                return Err(DriverError::MissingSigner);
            }
        }
        let mut proofs = Vec::with_capacity(intents.len());
        for intent in intents {
            proofs.push(self.proof(ChallengeRequest::Resume(intent.request))?);
        }
        self.check_budget()?;
        Ok(self.store.resume_pending_with_hook(
            proofs,
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?)
    }
    /// Explicit owner decision. Never spends resources or advances WorldTick.
    pub fn cancel_pending(
        &mut self,
        cause: MiniatureCancelCause,
    ) -> Result<nf_store::DurableAck, DriverError> {
        let proof = self.proof(ChallengeRequest::Cancel)?;
        self.check_budget()?;
        Ok(self.store.cancel_pending_with_hook(
            cause,
            proof,
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?)
    }
}
