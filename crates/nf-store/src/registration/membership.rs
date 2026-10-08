use super::{
    BranchRegistrar,
    model::{RegistrationError, Result},
};
use crate::schema::{counter, hash, read_counter};
use nf_identity::model::{IdentityError, MembershipRepository, MembershipState, Scope};
use rusqlite::{Connection, params};

pub(super) fn load(connection: &Connection, scope: Scope) -> Result<MembershipState> {
    let (revision, body, digest): (Vec<u8>, Vec<u8>, Vec<u8>) = connection.query_row(
        "SELECT revision,body,digest FROM registration_membership WHERE singleton=1",
        [],
        |row| {
            Ok((
                crate::persistence::bounded_blob(row, 0, 8)?,
                crate::persistence::bounded_blob(row, 1, 262144)?,
                crate::persistence::bounded_blob(row, 2, 32)?,
            ))
        },
    )?;
    if hash(&body) != digest.as_slice() {
        return Err(RegistrationError::Corrupt);
    }
    let state = nf_identity::codec::decode_state(&body)?;
    if state.scope != scope || state.revision != read_counter(&revision)? {
        return Err(RegistrationError::Corrupt);
    }
    Ok(state)
}
impl MembershipRepository for BranchRegistrar {
    fn load_membership(
        &mut self,
        scope: Scope,
    ) -> std::result::Result<Option<MembershipState>, IdentityError> {
        self.ensure().map_err(|_| IdentityError::Persistence)?;
        if scope != self.scope() {
            return Err(IdentityError::Scope);
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
        self.ensure().map_err(|_| IdentityError::Persistence)?;
        let scope = self.scope();
        commit(
            &mut self.connection,
            &mut self.quarantined,
            scope,
            expected,
            next,
        )
    }
}

/// Both owners guard their quarantine before delegating this same-connection transition.
pub(super) fn commit(
    connection: &mut Connection,
    quarantined: &mut bool,
    scope: Scope,
    expected: Option<u64>,
    next: &MembershipState,
) -> std::result::Result<(), IdentityError> {
    if next.scope != scope {
        return Err(IdentityError::Scope);
    }
    let expected = expected.ok_or(IdentityError::Conflict)?;
    if expected.checked_add(1) != Some(next.revision) {
        return Err(IdentityError::Frontier);
    }
    let body = nf_identity::codec::encode_state(next)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| IdentityError::Persistence)?;
    let current = load(&tx, scope).map_err(|_| IdentityError::Persistence)?;
    if current.revision != expected {
        return Err(IdentityError::Conflict);
    }
    tx.execute(
        "UPDATE registration_membership SET revision=?1,body=?2,digest=?3 WHERE singleton=1",
        params![counter(next.revision), body, hash(&body)],
    )
    .map_err(|_| IdentityError::Persistence)?;
    if tx.commit().is_err() {
        *quarantined = true;
        return Err(IdentityError::Persistence);
    }
    Ok(())
}
