use super::{codec, model::*};
use crate::registration::lease::{AdmissionPolicy, replay as lease_replay, schema as lease_schema};
use crate::registration::{RegistrationPolicy, schema as registration_schema};
use crate::{
    persistence::bounded_blob,
    schema::{counter, hash, read_counter},
};
use rusqlite::{Connection, Transaction, params};

const TAINT_SCHEMA: &str = "CREATE TABLE taint_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), head BLOB NOT NULL CHECK(length(head)=32), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), policy_body BLOB NOT NULL CHECK(length(policy_body)<=4096), registration_policy_digest BLOB NOT NULL CHECK(length(registration_policy_digest)=32), admission_policy_digest BLOB NOT NULL CHECK(length(admission_policy_digest)=32)) STRICT;\nCREATE TABLE taint_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), request BLOB NOT NULL UNIQUE CHECK(length(request)=16), previous BLOB NOT NULL CHECK(length(previous)=32), request_digest BLOB NOT NULL CHECK(length(request_digest)=32), head BLOB NOT NULL CHECK(length(head)=32), body BLOB NOT NULL CHECK(length(body)=733)) STRICT;\nCREATE TABLE taint_lineages (lineage BLOB PRIMARY KEY CHECK(length(lineage)=32), original_request BLOB NOT NULL UNIQUE CHECK(length(original_request)=16) REFERENCES taint_journal(request), revision BLOB NOT NULL UNIQUE CHECK(length(revision)=8)) STRICT;\n";
type MetaColumns = (
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
);
pub(super) struct Meta {
    pub revision: u64,
    pub head: [u8; 32],
}
pub(super) struct Profile {
    pub leases: lease_replay::State,
    pub taint: Option<Meta>,
}
fn full_schema() -> String {
    format!("{}{}", lease_schema::full_schema(), TAINT_SCHEMA)
}

pub(super) fn verify(
    connection: &Connection,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
    taint: &TaintPolicy,
) -> Result<Profile> {
    taint.validate(registration, admission)?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 2 {
        return Ok(Profile {
            leases: lease_replay::verify(connection, registration, admission)?,
            taint: None,
        });
    }
    if version != 3 {
        return Err(TaintError::UnsupportedProfile);
    }
    registration_schema::verify_shape(connection, 3, &full_schema())?;
    let base = lease_schema::from_validated_shape(connection, registration, admission, 3)?;
    let leases = lease_replay::from_validated_profile(connection, registration, admission, base)?;
    let (universe, history, revision, head, policy_digest, schema_digest, policy_body, registration_digest, admission_digest): MetaColumns =
        connection.query_row("SELECT universe,history,revision,head,policy_digest,schema_digest,policy_body,registration_policy_digest,admission_policy_digest FROM taint_meta WHERE singleton=1", [], |row| {
            Ok((bounded_blob(row, 0, 16)?, bounded_blob(row, 1, 16)?, bounded_blob(row, 2, 8)?,
                bounded_blob(row, 3, 32)?, bounded_blob(row, 4, 32)?, bounded_blob(row, 5, 32)?,
                bounded_blob(row, 6, 4096)?, bounded_blob(row, 7, 32)?, bounded_blob(row, 8, 32)?))
        })?;
    if universe != registration.scope.universe.as_bytes()
        || history != registration.scope.history.as_bytes()
        || policy_digest != taint.digest()?
        || schema_digest != hash(full_schema().as_bytes())
        || policy_body != codec::policy_bytes(taint)
        || registration_digest != registration.digest()?
        || admission_digest != admission.digest()
    {
        return Err(TaintError::Corrupt);
    }
    Ok(Profile {
        leases,
        taint: Some(Meta {
            revision: read_counter(&revision)?,
            head: head.try_into().map_err(|_| TaintError::Corrupt)?,
        }),
    })
}

/// Called only after a fully prepared first cause in its authenticated Immediate transaction.
/// Old registration/admission metadata and hashes are never rewritten.
pub(super) fn activate(
    tx: &Transaction<'_>,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
    taint: &TaintPolicy,
) -> Result<()> {
    taint.validate(registration, admission)?;
    let policy_body = codec::policy_bytes(taint);
    if policy_body.len() > 4096 {
        return Err(TaintError::Limit);
    }
    let policy_digest = taint.digest()?;
    let schema_digest = hash(full_schema().as_bytes());
    let registration_digest = registration.digest()?;
    let admission_digest = admission.digest();
    tx.execute_batch(TAINT_SCHEMA)?;
    tx.execute(
        "INSERT INTO taint_meta VALUES(1,?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            registration.scope.universe.as_bytes(),
            registration.scope.history.as_bytes(),
            counter(0),
            [0u8; 32],
            policy_digest,
            schema_digest,
            policy_body,
            registration_digest,
            admission_digest
        ],
    )?;
    tx.pragma_update(None, "user_version", 3)?;
    Ok(())
}
