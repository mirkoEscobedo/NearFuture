use super::{
    MiniatureStore, ProofAttempt,
    auth::Purpose,
    economic::{take_all, verify_intents},
    error::{MiniatureStoreError, Result},
};
use crate::{Accepted, Boundary, StoreError};
use nf_kernel::miniature::*;
impl MiniatureStore {
    pub fn resume_pending(&mut self, attempts: Vec<ProofAttempt>) -> Result<Accepted> {
        self.resume_pending_with_hook(attempts, &mut |_| Ok(()))
    }
    pub fn resume_pending_with_hook(
        &mut self,
        attempts: Vec<ProofAttempt>,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<Accepted> {
        self.ensure()?;
        let proofs = take_all(&mut self.runtime, attempts)?;
        if !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        let intents = self
            .pending()
            .ok_or(StoreError::InvalidTransition)?
            .intents()
            .to_vec();
        self.persist(
            1,
            |c, old| {
                let member = verify_intents(c, old, &intents, &proofs, Purpose::Resume)?;
                let a = old.authority.ok_or(MiniatureStoreError::Unclaimed)?;
                let frontier = admit_miniature(&old.world, intents.clone(), a.context(), member)?;
                Ok((
                    super::transitions::resume(old, &frontier)?,
                    super::transitions::frontier_action(3, &frontier)?,
                ))
            },
            |c, old, _| verify_intents(c, old, &intents, &proofs, Purpose::Resume).map(|_| ()),
            hook,
        )?;
        self.runtime.leases.clear();
        Ok(Accepted::new(self.revision))
    }
}
