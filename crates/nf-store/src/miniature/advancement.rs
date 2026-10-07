use super::{
    MiniatureStore, ProofAttempt,
    auth::{Evidence, Purpose, check},
    challenges::context,
    economic::current_authority,
    error::{MiniatureStoreError, Result},
    state::State,
};
use crate::{Boundary, DurableAck, StoreError, schema::hash};
use nf_kernel::miniature::*;
use rusqlite::Connection;
impl MiniatureStore {
    pub fn commit(&mut self, batch: &MiniatureBatch, attempt: ProofAttempt) -> Result<DurableAck> {
        self.commit_with_hook(batch, attempt, &mut |_| Ok(()))
    }
    pub fn commit_with_hook(
        &mut self,
        batch: &MiniatureBatch,
        attempt: ProofAttempt,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<DurableAck> {
        self.ensure()?;
        let evidence = self.runtime.take(attempt)?;
        if !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        if batch.disposition() != MiniatureDisposition::AdvanceTick {
            return Err(StoreError::InvalidTransition.into());
        }
        let leases = self.runtime.leases.clone();
        let validate = |c: &Connection, old: &State| -> Result<()> {
            let membership = verify_owner(c, old, &evidence, Purpose::Commit)?;
            let pending = old.pending.as_ref().ok_or(StoreError::InvalidTransition)?;
            let a = old.authority.ok_or(MiniatureStoreError::Unclaimed)?;
            if pending.membership_revision() != membership.revision
                || pending.authority() != a.context()
            {
                return Err(MiniatureStoreError::Fenced);
            }
            super::activity::verify_activity(&leases, old, &membership)
        };
        self.persist(
            2,
            |c, old| {
                validate(c, old)?;
                Ok((
                    super::transitions::commit(old, batch)?,
                    encode_miniature_batch(batch)?,
                ))
            },
            |c, old, _| validate(c, old),
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
pub(crate) fn verify_owner(
    c: &Connection,
    old: &State,
    e: &Evidence,
    purpose: Purpose,
) -> Result<nf_identity::model::MembershipState> {
    let membership = crate::identity::load(c, old.scope())?.ok_or(StoreError::Corrupt)?;
    let a = current_authority(old, &membership)?;
    let pending = old.pending.as_ref().ok_or(StoreError::InvalidTransition)?;
    let binding = hash(&encode_miniature_frontier(pending)?);
    let expected = context(
        old,
        &membership,
        purpose,
        a.account,
        a.device,
        binding,
        a.session,
    )?;
    check(e, expected, &membership, true)?;
    Ok(membership)
}
