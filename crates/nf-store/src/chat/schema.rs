use super::{
    ChatPolicy, ChatRatePolicy, ChatStoreError, KnownChatFrontiers, Result, codec, ledger,
    legacy_schema, rate_sql,
};
use nf_identity::model::{MembershipState, Scope};
use rusqlite::{Connection, params};
use std::collections::BTreeMap;
const APPLICATION_ID: i32 = 0x4e46_4348;
const VERSION: i32 = 3;
const SCHEMA: &str = "CREATE TABLE chat_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), policy_body BLOB NOT NULL CHECK(length(policy_body)=58), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32)) STRICT;\nCREATE TABLE membership (universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), public_state BLOB NOT NULL CHECK(length(public_state)<=262144), digest BLOB NOT NULL CHECK(length(digest)=32), PRIMARY KEY(universe,history)) STRICT;\nCREATE TABLE chat_messages (message BLOB PRIMARY KEY CHECK(length(message)=16), account BLOB NOT NULL CHECK(length(account)=16), device BLOB NOT NULL CHECK(length(device)=16), sequence BLOB NOT NULL CHECK(length(sequence)=8), receiver_cursor BLOB NOT NULL UNIQUE CHECK(length(receiver_cursor)=8), body BLOB NOT NULL CHECK(length(body)<=2176), signature BLOB NOT NULL CHECK(length(signature)=64), original_request BLOB NOT NULL UNIQUE CHECK(length(original_request)=16), UNIQUE(account,device,sequence)) STRICT;\nCREATE TABLE chat_requests (request BLOB PRIMARY KEY CHECK(length(request)=16), binding_digest BLOB NOT NULL CHECK(length(binding_digest)=32), message BLOB NOT NULL CHECK(length(message)=16) REFERENCES chat_messages(message)) STRICT;\n";
fn full_schema() -> String {
    format!("{SCHEMA}{}", rate_sql::SCHEMA)
}
type Meta = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
pub(super) fn initialize(
    connection: &mut Connection,
    policy: &ChatPolicy,
    membership: &MembershipState,
    rate: ChatRatePolicy,
) -> Result<()> {
    nf_identity::codec::validate_state(membership)?;
    if policy.scope != membership.scope {
        return Err(ChatStoreError::Scope);
    }
    let body = nf_identity::codec::encode_state(membership)?;
    let tx = connection.transaction()?;
    tx.execute_batch(&full_schema())?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    tx.execute(
        "INSERT INTO chat_meta VALUES(1,?1,?2,?3,?4,?5,?6)",
        params![
            policy.scope.universe.as_bytes(),
            policy.scope.history.as_bytes(),
            crate::schema::counter(0),
            codec::policy_bytes(policy),
            codec::policy_digest(policy),
            codec::hash(full_schema().as_bytes())
        ],
    )?;
    tx.execute(
        "INSERT INTO membership VALUES(?1,?2,?3,?4,?5)",
        params![
            policy.scope.universe.as_bytes(),
            policy.scope.history.as_bytes(),
            crate::schema::counter(membership.revision),
            body,
            codec::hash(&body)
        ],
    )?;
    rate_sql::initialize(&tx, policy, membership, rate, true)?;
    rate_sql::verify_all(&tx, membership)?;
    tx.commit()?;
    Ok(())
}
pub(super) fn current(connection: &Connection, scope: Scope) -> Result<MembershipState> {
    let state = crate::identity::load(connection, scope)?.ok_or(ChatStoreError::Corrupt)?;
    if state.scope != scope {
        return Err(ChatStoreError::Scope);
    }
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM membership LIMIT 2)",
        [],
        |r| r.get(0),
    )?;
    if count != 1 {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(state)
}
pub(super) fn verify(connection: &Connection, policy: &ChatPolicy) -> Result<ledger::State> {
    let app: i32 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if app != APPLICATION_ID || version != VERSION {
        return Err(ChatStoreError::UnsupportedProfile);
    }
    let integrity: String = connection.query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(ChatStoreError::Corrupt);
    }
    let extra: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))", [], |r| r.get(0))?;
    let complete_schema = full_schema();
    let expected: BTreeMap<_, _> = complete_schema
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|sql| {
            let name = sql
                .strip_prefix("CREATE TABLE ")
                .and_then(|s| s.split_whitespace().next())
                .expect("static schema");
            (name.to_owned(), sql.to_owned())
        })
        .collect();
    let mut statement = connection.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 7")?;
    let actual: BTreeMap<String, String> = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if extra || actual != expected {
        return Err(ChatStoreError::UnsupportedProfile);
    }
    let (universe, history, revision, policy_body, policy_digest, schema_digest): Meta = connection.query_row("SELECT universe,history,revision,policy_body,policy_digest,schema_digest FROM chat_meta WHERE singleton=1", [], |r| Ok((crate::persistence::bounded_blob(r,0,16)?,crate::persistence::bounded_blob(r,1,16)?,crate::persistence::bounded_blob(r,2,8)?,crate::persistence::bounded_blob(r,3,58)?,crate::persistence::bounded_blob(r,4,32)?,crate::persistence::bounded_blob(r,5,32)?)))?;
    if universe != policy.scope.universe.as_bytes() || history != policy.scope.history.as_bytes() {
        return Err(ChatStoreError::Scope);
    }
    if policy_body != codec::policy_bytes(policy)
        || policy_digest != codec::policy_digest(policy)
        || schema_digest != codec::hash(full_schema().as_bytes())
    {
        return Err(ChatStoreError::Corrupt);
    }
    let revision = crate::schema::read_counter(&revision)?;
    let membership = current(connection, policy.scope)?;
    ledger::verify(connection, policy, revision, &membership)
}

pub(super) fn migrate_v2(
    connection: &mut Connection,
    policy: &ChatPolicy,
    known: KnownChatFrontiers,
    rate: ChatRatePolicy,
) -> Result<rate_sql::Snapshot> {
    if known.scope != policy.scope {
        return Err(ChatStoreError::Scope);
    }
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let state = legacy_schema::verify(&tx, policy)?;
    let members = legacy_schema::current(&tx, policy.scope)?;
    if state.revision < known.revision || members.revision < known.membership_revision {
        return Err(ChatStoreError::StaleBackup);
    }
    tx.execute_batch(rate_sql::SCHEMA)?;
    #[cfg(test)]
    super::rate_crash_controls::cut("migration-after-ddl");
    rate_sql::initialize(&tx, policy, &members, rate, false)?;
    #[cfg(test)]
    super::rate_crash_controls::cut("migration-after-rows");
    if tx.execute(
        "UPDATE chat_meta SET schema_digest=?1 WHERE singleton=1 AND schema_digest=?2",
        params![
            codec::hash(full_schema().as_bytes()),
            codec::hash(SCHEMA.as_bytes())
        ],
    )? != 1
    {
        return Err(ChatStoreError::Conflict);
    }
    tx.pragma_update(None, "user_version", VERSION)?;
    #[cfg(test)]
    super::rate_crash_controls::cut("migration-after-version");
    verify(&tx, policy)?;
    let snapshot = rate_sql::verify_all(&tx, &members)?;
    tx.commit()?;
    #[cfg(test)]
    super::rate_crash_controls::cut("migration-after-commit");
    Ok(snapshot)
}
