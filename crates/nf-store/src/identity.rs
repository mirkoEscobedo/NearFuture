use crate::{
    Store,
    error::{Result, StoreError},
    model::KnownFrontiers,
    persistence::bounded_blob,
    schema::{counter, hash, read_counter},
};
use nf_identity::{
    codec::{decode_state, encode_state},
    model::{IdentityError, MembershipRepository, MembershipState, Scope},
};
use rusqlite::{Connection, OptionalExtension, params};
pub(crate) fn load(connection: &Connection, scope: Scope) -> Result<Option<MembershipState>> {
    let record = connection
        .query_row(
            "SELECT revision,public_state,digest FROM membership WHERE universe=?1 AND history=?2",
            params![scope.universe.as_bytes(), scope.history.as_bytes()],
            |row| {
                Ok((
                    bounded_blob(row, 0, 8)?,
                    bounded_blob(row, 1, 262_144)?,
                    bounded_blob(row, 2, 32)?,
                ))
            },
        )
        .optional()?;
    let Some((revision, body, digest)) = record else {
        return Ok(None);
    };
    if hash(&body) != digest.as_slice() {
        return Err(StoreError::Corrupt);
    }
    let state = decode_state(&body).map_err(|_| StoreError::Corrupt)?;
    if state.scope != scope || state.revision != read_counter(&revision)? {
        return Err(StoreError::Corrupt);
    }
    Ok(Some(state))
}
pub(crate) fn verify_minimum(connection: &Connection, known: KnownFrontiers) -> Result<()> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM membership LIMIT 2)",
        [],
        |row| row.get(0),
    )?;
    let state = load(connection, known.scope)?;
    if count != i64::from(state.is_some()) {
        return Err(StoreError::Scope);
    }
    if let Some(minimum) = known.membership_revision
        && state.is_none_or(|state| state.revision < minimum)
    {
        return Err(StoreError::StaleBackup);
    }
    Ok(())
}
pub(crate) fn revision(connection: &Connection, scope: Scope) -> Result<Option<u64>> {
    Ok(load(connection, scope)?.map(|state| state.revision))
}
impl MembershipRepository for Store {
    fn load_membership(
        &mut self,
        scope: Scope,
    ) -> std::result::Result<Option<MembershipState>, IdentityError> {
        self.ensure_writable()
            .map_err(|_| IdentityError::Persistence)?;
        if scope != self.state.scope() {
            return Err(IdentityError::Scope);
        }
        load(&self.connection, scope).map_err(|_| IdentityError::Persistence)
    }
    fn commit_membership(
        &mut self,
        expected_revision: Option<u64>,
        next: &MembershipState,
    ) -> std::result::Result<(), IdentityError> {
        self.ensure_writable()
            .map_err(|_| IdentityError::Persistence)?;
        if next.scope != self.state.scope() {
            return Err(IdentityError::Scope);
        }
        let revision = match expected_revision {
            None => 0,
            Some(value) => value.checked_add(1).ok_or(IdentityError::Frontier)?,
        };
        if next.revision != revision {
            return Err(IdentityError::Frontier);
        }
        let body = encode_state(next)?;
        let transaction = self
            .connection
            .transaction()
            .map_err(|_| IdentityError::Persistence)?;
        let current = load(&transaction, next.scope).map_err(|_| IdentityError::Persistence)?;
        if current.as_ref().map(|state| state.revision) != expected_revision {
            return Err(IdentityError::Conflict);
        }
        transaction.execute("INSERT INTO membership VALUES (?1,?2,?3,?4,?5) ON CONFLICT(universe,history) DO UPDATE SET revision=excluded.revision,public_state=excluded.public_state,digest=excluded.digest",params![next.scope.universe.as_bytes(),next.scope.history.as_bytes(),counter(next.revision),body,hash(&body)]).map_err(|_|IdentityError::Persistence)?;
        if transaction.commit().is_err() {
            self.quarantined = true;
            return Err(IdentityError::Persistence);
        }
        Ok(())
    }
}
