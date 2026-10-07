use super::{
    MiniatureStore, ProofAttempt,
    auth::{Evidence, Purpose, check},
    challenges::context,
    economic::current_authority,
    error::{MiniatureStoreError, Result},
    state::State,
};
use crate::StoreError;
use nf_identity::model::MembershipState;
use std::time::Instant;
impl MiniatureStore {
    pub fn accept_activity(&mut self, attempt: ProofAttempt) -> Result<()> {
        self.ensure()?;
        let evidence = self.runtime.take(attempt)?;
        if !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        self.runtime.leases.retain(|e| Instant::now() < e.deadline);
        if self.runtime.leases.len() >= 64 {
            return Err(StoreError::Backpressure.into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        super::persistence::verify_current(&tx, &self.state, self.revision, self.head)?;
        let membership =
            crate::identity::load(&tx, self.state.scope())?.ok_or(StoreError::Corrupt)?;
        verify_lease(&evidence, &self.state, &membership)?;
        if let Err(error) = tx.commit() {
            self.quarantined = true;
            self.runtime.clear();
            self.runtime.claimed = false;
            return Err(error.into());
        }
        self.runtime.leases.push(evidence);
        Ok(())
    }
}
pub(crate) fn verify_lease(e: &Evidence, old: &State, membership: &MembershipState) -> Result<()> {
    let a = current_authority(old, membership)?;
    if !old
        .world
        .component()
        .genesis()
        .accounts
        .contains(&e.context.actor)
    {
        return Err(MiniatureStoreError::Unauthorized);
    }
    let expected = context(
        old,
        membership,
        Purpose::Lease,
        e.context.actor,
        e.context.device,
        [0; 32],
        a.session,
    )?;
    check(e, expected, membership, false)
}
pub(crate) fn verify_activity(
    leases: &[Evidence],
    old: &State,
    membership: &MembershipState,
) -> Result<()> {
    if leases
        .iter()
        .any(|e| verify_lease(e, old, membership).is_ok())
    {
        return Ok(());
    }
    Err(MiniatureStoreError::NoActivity)
}
