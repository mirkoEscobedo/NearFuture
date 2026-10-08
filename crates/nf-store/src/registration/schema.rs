use super::{
    codec::policy_bytes,
    model::{RegistrationError, RegistrationPolicy, Result},
};
use crate::schema::{counter, hash, read_counter};
use nf_identity::model::MembershipState;
use rusqlite::{Connection, params};

const APPLICATION_ID: i32 = 0x4e46_4341;
pub(super) const SCHEMA: &str = "CREATE TABLE registration_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), head BLOB NOT NULL CHECK(length(head)=32), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), policy_body BLOB NOT NULL CHECK(length(policy_body)<=4096)) STRICT;\nCREATE TABLE registration_membership (singleton INTEGER PRIMARY KEY CHECK(singleton=1), revision BLOB NOT NULL CHECK(length(revision)=8), body BLOB NOT NULL CHECK(length(body)<=262144), digest BLOB NOT NULL CHECK(length(digest)=32)) STRICT;\nCREATE TABLE registration_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), request BLOB NOT NULL UNIQUE CHECK(length(request)=16), previous BLOB NOT NULL CHECK(length(previous)=32), request_digest BLOB NOT NULL CHECK(length(request_digest)=32), head BLOB NOT NULL CHECK(length(head)=32), body BLOB NOT NULL CHECK(length(body)<=256)) STRICT;\nCREATE TABLE registration_branches (campaign BLOB NOT NULL CHECK(length(campaign)=16), branch BLOB NOT NULL CHECK(length(branch)=16), account BLOB NOT NULL CHECK(length(account)=16), original_request BLOB NOT NULL CHECK(length(original_request)=16) REFERENCES registration_journal(request), revision BLOB NOT NULL UNIQUE CHECK(length(revision)=8), PRIMARY KEY(campaign,branch)) STRICT;\n";
type MetaColumns = (
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
);

pub(super) fn initialize(
    connection: &mut Connection,
    policy: &RegistrationPolicy,
    membership: &MembershipState,
) -> Result<()> {
    let body = nf_identity::codec::encode_state(membership)?;
    let config = policy_bytes(policy)?;
    let tx = connection.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", 1)?;
    tx.execute(
        "INSERT INTO registration_meta VALUES(1,?1,?2,?3,?4,?5,?6,?7)",
        params![
            policy.scope.universe.as_bytes(),
            policy.scope.history.as_bytes(),
            counter(0),
            [0u8; 32],
            policy.digest()?,
            hash(SCHEMA.as_bytes()),
            config
        ],
    )?;
    tx.execute(
        "INSERT INTO registration_membership VALUES(1,?1,?2,?3)",
        params![counter(membership.revision), body, hash(&body)],
    )?;
    tx.commit()?;
    Ok(())
}

pub(super) struct Meta {
    pub revision: u64,
    pub head: [u8; 32],
}

pub(super) fn verify(connection: &Connection, policy: &RegistrationPolicy) -> Result<Meta> {
    verify_shape(connection, 1, SCHEMA)?;
    read_meta(connection, policy)
}

/// Private exact-profile seam; the standalone wrapper always selects four tables/version one.
pub(super) fn verify_shape(
    connection: &Connection,
    expected_version: i32,
    expected_schema: &str,
) -> Result<()> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if id != APPLICATION_ID || version != expected_version {
        return Err(RegistrationError::UnsupportedProfile);
    }
    let integrity: String =
        connection.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(RegistrationError::Corrupt);
    }
    let expected: std::collections::BTreeMap<_, _> = expected_schema
        .split(';')
        .map(str::trim)
        .filter(|sql| !sql.is_empty())
        .map(|sql| {
            (
                sql.strip_prefix("CREATE TABLE ")
                    .and_then(|sql| sql.split_whitespace().next())
                    .expect("static schema table")
                    .to_owned(),
                sql.to_owned(),
            )
        })
        .collect();
    let extra: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type<>'table' AND (type<>'index' OR sql IS NOT NULL OR name NOT LIKE 'sqlite_autoindex_%'))", [], |row| row.get(0))?;
    let query = match expected_version {
        1 => {
            "SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 5"
        }
        2 => {
            "SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 8"
        }
        _ => return Err(RegistrationError::UnsupportedProfile),
    };
    let mut statement = connection.prepare(query)?;
    let actual: std::collections::BTreeMap<String, String> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if extra || actual != expected {
        return Err(RegistrationError::UnsupportedProfile);
    }
    Ok(())
}

/// Caller has validated its complete selected schema before reading registration metadata.
pub(super) fn read_meta(connection: &Connection, policy: &RegistrationPolicy) -> Result<Meta> {
    let (universe, history, revision, head, stored_policy, schema, config): MetaColumns = connection.query_row(
        "SELECT universe,history,revision,head,policy_digest,schema_digest,policy_body FROM registration_meta WHERE singleton=1", [], |row| {
            Ok((crate::persistence::bounded_blob(row, 0, 16)?, crate::persistence::bounded_blob(row, 1, 16)?,
                crate::persistence::bounded_blob(row, 2, 8)?, crate::persistence::bounded_blob(row, 3, 32)?,
                crate::persistence::bounded_blob(row, 4, 32)?, crate::persistence::bounded_blob(row, 5, 32)?,
                crate::persistence::bounded_blob(row, 6, 4096)?))
        })?;
    if universe != policy.scope.universe.as_bytes()
        || history != policy.scope.history.as_bytes()
        || stored_policy != policy.digest()?
        || schema != hash(SCHEMA.as_bytes())
        || config != policy_bytes(policy)?
    {
        return Err(RegistrationError::Corrupt);
    }
    Ok(Meta {
        revision: read_counter(&revision)?,
        head: super::codec::fixed(&head)?,
    })
}
