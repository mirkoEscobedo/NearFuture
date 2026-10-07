use super::{
    MiniatureStore,
    auth::{ProofAttempt, Purpose, check},
    challenges::context,
    error::Result,
    model::*,
};
use crate::{Accepted, Boundary, StoreError};
impl MiniatureStore {
    pub fn claim_authority(&mut self, attempt: ProofAttempt) -> Result<Accepted> {
        self.claim_authority_with_hook(attempt, &mut |_| Ok(()))
    }
    pub fn claim_authority_with_hook(
        &mut self,
        attempt: ProofAttempt,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<Accepted> {
        self.ensure()?;
        let evidence = self.runtime.take(attempt)?;
        let validate = |c: &rusqlite::Connection,
                        old: &super::state::State|
         -> Result<MiniatureAuthority> {
            if !old
                .world
                .component()
                .genesis()
                .accounts
                .contains(&evidence.context.actor)
            {
                return Err(super::error::MiniatureStoreError::Unauthorized);
            }
            let membership = crate::identity::load(c, old.scope())?.ok_or(StoreError::Corrupt)?;
            let expected = context(
                old,
                &membership,
                Purpose::Claim,
                evidence.context.actor,
                evidence.context.device,
                [0; 32],
                evidence.context.session,
            )?;
            check(&evidence, expected, &membership, true)?;
            Ok(MiniatureAuthority {
                term: expected.proposed_term,
                session: expected.session,
                account: expected.actor,
                device: expected.device,
                membership_revision: membership.revision,
            })
        };
        self.persist(
            1,
            |c, old| {
                let a = validate(c, old)?;
                Ok((
                    super::transitions::claim(old, a)?,
                    super::transitions::claim_action(a),
                ))
            },
            |c, old, _| validate(c, old).map(|_| ()),
            hook,
        )?;
        self.runtime.clear();
        self.runtime.claimed = true;
        Ok(Accepted::new(self.revision))
    }
}
