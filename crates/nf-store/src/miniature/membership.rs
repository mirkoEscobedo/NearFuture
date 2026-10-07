use super::MiniatureStore;
use crate::schema::{counter, hash};
use nf_identity::{
    codec::encode_state,
    model::{IdentityError, MembershipRepository, MembershipState, Scope},
};
use rusqlite::params;
impl MembershipRepository for MiniatureStore {
    fn load_membership(&mut self, scope: Scope) -> Result<Option<MembershipState>, IdentityError> {
        self.ensure().map_err(|_| IdentityError::Persistence)?;
        if scope != self.state.scope() {
            return Err(IdentityError::Scope);
        }
        crate::identity::load(&self.connection, scope).map_err(|_| IdentityError::Persistence)
    }
    fn commit_membership(
        &mut self,
        expected_revision: Option<u64>,
        next: &MembershipState,
    ) -> Result<(), IdentityError> {
        self.ensure().map_err(|_| IdentityError::Persistence)?;
        if next.scope != self.state.scope() {
            return Err(IdentityError::Scope);
        }
        let revision = match expected_revision {
            None => 0,
            Some(v) => v.checked_add(1).ok_or(IdentityError::Frontier)?,
        };
        if next.revision != revision {
            return Err(IdentityError::Frontier);
        }
        let bytes = encode_state(next)?;
        let tx = self
            .connection
            .transaction()
            .map_err(|_| IdentityError::Persistence)?;
        let current =
            crate::identity::load(&tx, next.scope).map_err(|_| IdentityError::Persistence)?;
        if current.as_ref().map(|s| s.revision) != expected_revision {
            return Err(IdentityError::Conflict);
        }
        tx.execute("INSERT INTO membership VALUES (?1,?2,?3,?4,?5) ON CONFLICT(universe,history) DO UPDATE SET revision=excluded.revision,public_state=excluded.public_state,digest=excluded.digest",params![next.scope.universe.as_bytes(),next.scope.history.as_bytes(),counter(next.revision),bytes,hash(&bytes)]).map_err(|_|IdentityError::Persistence)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            self.runtime.clear();
            self.runtime.claimed = false;
            return Err(IdentityError::Persistence);
        }
        self.runtime.clear();
        Ok(())
    }
}
