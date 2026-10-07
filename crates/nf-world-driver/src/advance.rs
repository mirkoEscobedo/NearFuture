use crate::{Driver, DriverError};
use nf_store::{
    DurableAck, StoreError,
    miniature::{ChallengeRequest, MiniatureStoreError},
};
impl Driver {
    pub fn advance_pending(&mut self, active: bool) -> Result<DurableAck, DriverError> {
        if !active {
            return Err(DriverError::Inactive);
        }
        if self.store.pending().is_none() {
            return Err(StoreError::InvalidTransition.into());
        }
        self.activity()?;
        self.commit_pending()
    }
    pub fn advance_empty(&mut self, active: bool) -> Result<DurableAck, DriverError> {
        self.advance_empty_observed(active, &mut |_| {})
    }
    pub(super) fn advance_empty_observed(
        &mut self,
        active: bool,
        observer: &mut impl FnMut(crate::RunStage),
    ) -> Result<DurableAck, DriverError> {
        if !active {
            return Err(DriverError::Inactive);
        }
        if self.store.pending().is_some() {
            return Err(StoreError::InvalidTransition.into());
        }
        self.activity()?;
        self.check_budget()?;
        self.store.prepare_with_hook(
            Vec::new(),
            Vec::new(),
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?;
        observer(crate::RunStage::PendingPrepared);
        self.commit_pending()
    }
    fn activity(&mut self) -> Result<(), DriverError> {
        let public = self.signer.public();
        let attempt = self.proof(ChallengeRequest::Activity {
            actor: public.account,
            device: public.device,
        })?;
        self.check_budget()?;
        self.store.accept_activity(attempt)?;
        self.check_budget()?;
        Ok(())
    }
    fn commit_pending(&mut self) -> Result<DurableAck, DriverError> {
        let authority = self
            .store
            .authority()
            .ok_or(MiniatureStoreError::Unclaimed)?;
        let frontier = self.store.pending().ok_or(StoreError::InvalidTransition)?;
        let batch = nf_kernel::miniature::settle_miniature(
            self.store.world(),
            frontier,
            authority.context(),
        )
        .map_err(MiniatureStoreError::Kernel)?;
        let attempt = self.proof(ChallengeRequest::Commit)?;
        self.check_budget()?;
        Ok(self.store.commit_with_hook(
            &batch,
            attempt,
            &mut crate::run_budget::deadline_hook(self.run_deadline),
        )?)
    }
}
