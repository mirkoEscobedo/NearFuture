use super::error::{Result, SuppliesStoreError};
use crate::schema::{counter, hash, read_counter};
use nf_identity::model::{MembershipState, Scope};
use nf_kernel::supplies::{SuppliesPolicy, policy_bytes, policy_digest};
use rusqlite::{Connection, params};
type MetaColumns = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
pub(super) const APPLICATION_ID: i32 = 0x4e46_5355;
pub(super) const SCHEMA: &str = "CREATE TABLE supplies_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), policy_body BLOB NOT NULL CHECK(length(policy_body)<=8192)) STRICT;\nCREATE TABLE supplies_membership (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), body BLOB NOT NULL CHECK(length(body)<=262144), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;\nCREATE TABLE supplies_operations (operation BLOB PRIMARY KEY CHECK(length(operation)=16), economic_digest BLOB NOT NULL CHECK(length(economic_digest)=32), body BLOB NOT NULL CHECK(length(body)<=384), revision BLOB NOT NULL UNIQUE CHECK(length(revision)=8), decision INTEGER NOT NULL CHECK(decision IN(0,1))) STRICT;\nCREATE TABLE supplies_requests (request BLOB PRIMARY KEY CHECK(length(request)=16), binding_digest BLOB NOT NULL CHECK(length(binding_digest)=32), operation BLOB NOT NULL CHECK(length(operation)=16) REFERENCES supplies_operations(operation), body BLOB NOT NULL CHECK(length(body)<=384)) STRICT;\nCREATE TABLE supplies_balances (account BLOB NOT NULL CHECK(length(account)=16), content BLOB NOT NULL CHECK(length(content)=32), origin BLOB NOT NULL CHECK(length(origin)=36), body BLOB NOT NULL CHECK(length(body)=48), PRIMARY KEY(account,content,origin)) STRICT;\n";
pub(super) fn initialize(
    connection: &mut Connection,
    scope: Scope,
    policy: &SuppliesPolicy,
    membership: &MembershipState,
) -> Result<()> {
    let body = nf_identity::codec::encode_state(membership)?;
    let config = policy_bytes(policy)?;
    let tx = connection.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", 4)?;
    tx.execute(
        "INSERT INTO supplies_meta VALUES(1,?1,?2,?3,?4,?5,?6)",
        params![
            scope.universe.as_bytes(),
            scope.history.as_bytes(),
            counter(0),
            policy_digest(policy)?,
            hash(SCHEMA.as_bytes()),
            config
        ],
    )?;
    tx.execute(
        "INSERT INTO supplies_membership VALUES(1,?1,?2,?3)",
        params![counter(membership.revision), body, hash(&body)],
    )?;
    tx.commit()?;
    Ok(())
}
pub(super) fn verify(
    connection: &Connection,
    scope: Scope,
    policy: &SuppliesPolicy,
) -> Result<u64> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if id != APPLICATION_ID || version != 4 {
        return Err(SuppliesStoreError::UnsupportedProfile);
    }
    let integrity: String = connection.query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(SuppliesStoreError::Corrupt);
    }
    let expected: std::collections::BTreeMap<_, _> = SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            (
                s.strip_prefix("CREATE TABLE ")
                    .and_then(|s| s.split_whitespace().next())
                    .expect("static table")
                    .to_owned(),
                s.to_owned(),
            )
        })
        .collect();
    let extra: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))", [], |r|r.get(0))?;
    let mut statement = connection.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 6")?;
    let actual: std::collections::BTreeMap<String, String> = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if extra || actual != expected {
        return Err(SuppliesStoreError::UnsupportedProfile);
    }
    let (universe,history,revision,stored_policy,schema,config): MetaColumns = connection.query_row("SELECT universe,history,revision,policy_digest,schema_digest,policy_body FROM supplies_meta WHERE singleton=1", [], |r|Ok((crate::persistence::bounded_blob(r,0,16)?,crate::persistence::bounded_blob(r,1,16)?,crate::persistence::bounded_blob(r,2,8)?,crate::persistence::bounded_blob(r,3,32)?,crate::persistence::bounded_blob(r,4,32)?,crate::persistence::bounded_blob(r,5,8192)?)))?;
    if universe != scope.universe.as_bytes()
        || history != scope.history.as_bytes()
        || stored_policy != policy_digest(policy)?
        || schema != hash(SCHEMA.as_bytes())
        || config != policy_bytes(policy)?
    {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(read_counter(&revision)?)
}
