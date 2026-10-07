use super::{
    MiniatureStore,
    auth::{AuthRuntime, Evidence, ProofAttempt, Purpose, check},
    challenges::context,
    error::{MiniatureStoreError, Result},
    model::*,
    state::State,
};
use crate::{Accepted, Boundary, StoreError, schema::hash};
use nf_kernel::miniature::*;
use rusqlite::Connection;
impl MiniatureStore {
    pub fn prepare(
        &mut self,
        intents: Vec<MiniatureIntent>,
        attempts: Vec<ProofAttempt>,
    ) -> Result<Accepted> {
        self.prepare_with_hook(intents, attempts, &mut |_| Ok(()))
    }
    pub fn prepare_with_hook(
        &mut self,
        intents: Vec<MiniatureIntent>,
        attempts: Vec<ProofAttempt>,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<Accepted> {
        self.ensure()?;
        let proofs = take_all(&mut self.runtime, attempts)?;
        if !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        if intents.len() > 64 {
            return Err(StoreError::Limit.into());
        }
        self.persist(
            1,
            |c, old| {
                let member = verify_intents(c, old, &intents, &proofs, Purpose::Prepare)?;
                let a = old.authority.ok_or(MiniatureStoreError::Unclaimed)?;
                let frontier = admit_miniature(&old.world, intents.clone(), a.context(), member)?;
                Ok((
                    super::transitions::prepare(old, &frontier)?,
                    super::transitions::frontier_action(1, &frontier)?,
                ))
            },
            |c, old, _| verify_intents(c, old, &intents, &proofs, Purpose::Prepare).map(|_| ()),
            hook,
        )?;
        Ok(Accepted::new(self.revision))
    }
    /// Data only. The local owner must authenticate and authorize its configured signer first.
    pub fn query_bound(
        &self,
        request: nf_contract::identity::RequestId,
        account: nf_contract::identity::AccountId,
        device: nf_contract::identity::DeviceId,
        scope: nf_identity::model::Scope,
    ) -> Result<Option<MiniatureBoundRequest>> {
        self.ensure()?;
        if scope != self.state.scope() {
            return Err(StoreError::Scope.into());
        }
        let Some(r) = self.state.requests.get(&request) else {
            return Ok(None);
        };
        if r.intent.actor != account || r.intent.device != device {
            return Err(StoreError::RequestConflict.into());
        }
        Ok(Some(r.clone()))
    }
}
pub(crate) fn take_all(
    runtime: &mut AuthRuntime,
    attempts: Vec<ProofAttempt>,
) -> Result<Vec<Evidence>> {
    if attempts.len() > 64 {
        runtime.clear();
        return Err(StoreError::Limit.into());
    }
    let mut proofs = Vec::with_capacity(attempts.len());
    let mut failure = None;
    for attempt in attempts {
        match runtime.take(attempt) {
            Ok(e) => proofs.push(e),
            Err(e) => {
                failure.get_or_insert(e);
            }
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(proofs)
}
pub(crate) fn current_authority(
    old: &State,
    membership: &nf_identity::model::MembershipState,
) -> Result<MiniatureAuthority> {
    let a = old.authority.ok_or(MiniatureStoreError::Unclaimed)?;
    super::auth::identity(membership, a.account, a.device)?;
    let roles = membership.accounts[&a.account].roles;
    if !old
        .world
        .component()
        .genesis()
        .accounts
        .contains(&a.account)
        || !roles.contains(nf_identity::model::Roles::PLAYER)
        || !roles.contains(nf_identity::model::Roles::AUTHORITY_CANDIDATE)
    {
        return Err(MiniatureStoreError::Unauthorized);
    }
    Ok(a)
}
pub(crate) fn verify_intents(
    c: &Connection,
    old: &State,
    intents: &[MiniatureIntent],
    proofs: &[Evidence],
    purpose: Purpose,
) -> Result<u64> {
    if intents.len() != proofs.len() {
        return Err(MiniatureStoreError::Unauthorized);
    }
    let membership = crate::identity::load(c, old.scope())?.ok_or(StoreError::Corrupt)?;
    let a = current_authority(old, &membership)?;
    for i in intents {
        let binding = hash(&encode_miniature_intent(i)?);
        let e = proofs
            .iter()
            .find(|e| {
                e.context.actor == i.actor
                    && e.context.device == i.device
                    && e.context.binding == binding
            })
            .ok_or(MiniatureStoreError::Unauthorized)?;
        let expected = context(
            old,
            &membership,
            purpose,
            i.actor,
            i.device,
            binding,
            a.session,
        )?;
        check(e, expected, &membership, false)?;
    }
    Ok(membership.revision)
}
