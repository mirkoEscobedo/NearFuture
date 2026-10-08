use super::{
    SuppliesStore,
    error::{Result, SuppliesStoreError},
};
use crate::schema::{counter, hash, read_counter};
use nf_identity::model::{IdentityError, MembershipRepository, MembershipState, Scope};
use rusqlite::{Connection, params};
pub(crate) fn load(connection: &Connection, scope: Scope) -> Result<MembershipState> {
    let (revision, body, digest): (Vec<u8>, Vec<u8>, Vec<u8>) = connection.query_row(
        "SELECT revision,body,digest FROM supplies_membership WHERE singleton=1",
        [],
        |r| {
            Ok((
                crate::persistence::bounded_blob(r, 0, 8)?,
                crate::persistence::bounded_blob(r, 1, 262144)?,
                crate::persistence::bounded_blob(r, 2, 32)?,
            ))
        },
    )?;
    if hash(&body) != digest.as_slice() {
        return Err(SuppliesStoreError::Corrupt);
    }
    let state = nf_identity::codec::decode_state(&body)?;
    if state.scope != scope || state.revision != read_counter(&revision)? {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(state)
}
impl MembershipRepository for SuppliesStore {
    fn load_membership(
        &mut self,
        scope: Scope,
    ) -> std::result::Result<Option<MembershipState>, IdentityError> {
        if scope != self.scope() {
            return Err(IdentityError::Scope);
        }
        if self.quarantined {
            return Err(IdentityError::Persistence);
        }
        load(&self.connection, scope)
            .map(Some)
            .map_err(|_| IdentityError::Persistence)
    }
    fn commit_membership(
        &mut self,
        expected: Option<u64>,
        next: &MembershipState,
    ) -> std::result::Result<(), IdentityError> {
        if self.quarantined {
            return Err(IdentityError::Persistence);
        }
        if next.scope != self.scope() {
            return Err(IdentityError::Scope);
        }
        let expected = expected.ok_or(IdentityError::Conflict)?;
        if expected.checked_add(1) != Some(next.revision) {
            return Err(IdentityError::Frontier);
        }
        let body = nf_identity::codec::encode_state(next)?;
        let scope = self.scope();
        let tx = self
            .connection
            .transaction()
            .map_err(|_| IdentityError::Persistence)?;
        let current = load(&tx, scope).map_err(|_| IdentityError::Persistence)?;
        if current.revision != expected {
            return Err(IdentityError::Conflict);
        }
        tx.execute(
            "UPDATE supplies_membership SET revision=?1,body=?2,digest=?3 WHERE singleton=1",
            params![counter(next.revision), body, hash(&body)],
        )
        .map_err(|_| IdentityError::Persistence)?;
        if tx.commit().is_err() {
            self.quarantined = true;
            return Err(IdentityError::Persistence);
        }
        Ok(())
    }
}
