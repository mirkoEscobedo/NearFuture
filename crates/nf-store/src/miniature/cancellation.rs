use super::{
    MiniatureCancelCause, MiniatureStore, ProofAttempt,
    auth::Purpose,
    error::{MiniatureStoreError, Result},
};
use crate::{Boundary, DurableAck, StoreError};
use nf_kernel::miniature::*;
impl MiniatureStore {
    pub fn cancel_pending(
        &mut self,
        cause: MiniatureCancelCause,
        attempt: ProofAttempt,
    ) -> Result<DurableAck> {
        self.cancel_pending_with_hook(cause, attempt, &mut |_| Ok(()))
    }
    pub fn cancel_pending_with_hook(
        &mut self,
        cause: MiniatureCancelCause,
        attempt: ProofAttempt,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<DurableAck> {
        self.ensure()?;
        let evidence = self.runtime.take(attempt)?;
        if !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        let reason = match cause {
            MiniatureCancelCause::AdmissionChanged => MiniatureRejection::AdmissionChanged,
            MiniatureCancelCause::Cancelled => MiniatureRejection::Cancelled,
        };
        self.persist(
            2,
            |c, old| {
                let membership =
                    super::advancement::verify_owner(c, old, &evidence, Purpose::Cancel)?;
                let frontier = old.pending.as_ref().ok_or(StoreError::InvalidTransition)?;
                let a = old.authority.ok_or(MiniatureStoreError::Unclaimed)?;
                if cause == MiniatureCancelCause::AdmissionChanged
                    && membership.revision == frontier.membership_revision()
                    && a.context() == frontier.authority()
                {
                    return Err(StoreError::InvalidTransition.into());
                }
                let batch = cancel_miniature(
                    &old.world,
                    frontier,
                    a.context(),
                    membership.revision,
                    reason,
                )?;
                Ok((
                    super::transitions::commit(old, &batch)?,
                    encode_miniature_batch(&batch)?,
                ))
            },
            |c, old, _| {
                super::advancement::verify_owner(c, old, &evidence, Purpose::Cancel).map(|_| ())
            },
            hook,
        )?;
        self.runtime.leases.clear();
        Ok(DurableAck {
            revision: self.revision,
            sequence: self.world().metadata().event_sequence,
            state_hash: miniature_state_hash(self.world())?,
        })
    }
}
