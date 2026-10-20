use super::{codec, model::*};
use crate::{
    persistence::bounded_blob,
    registration::{
        RegistrationPolicy, replay as registration_replay, schema as registration_schema,
    },
    schema::{counter, hash, read_counter},
};
use rusqlite::{Connection, Transaction, params};

const LEASE_SCHEMA: &str = "CREATE TABLE admission_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), revision BLOB NOT NULL CHECK(length(revision)=8), head BLOB NOT NULL CHECK(length(head)=32), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), policy_body BLOB NOT NULL CHECK(length(policy_body)=60), registration_policy_digest BLOB NOT NULL CHECK(length(registration_policy_digest)=32)) STRICT;\nCREATE TABLE admission_journal (revision BLOB PRIMARY KEY CHECK(length(revision)=8), request BLOB NOT NULL UNIQUE CHECK(length(request)=16), previous BLOB NOT NULL CHECK(length(previous)=32), request_digest BLOB NOT NULL CHECK(length(request_digest)=32), head BLOB NOT NULL CHECK(length(head)=32), body BLOB NOT NULL CHECK(length(body)=452)) STRICT;\nCREATE TABLE admission_active (campaign BLOB NOT NULL CHECK(length(campaign)=16), branch BLOB NOT NULL CHECK(length(branch)=16), account BLOB NOT NULL CHECK(length(account)=16), device BLOB NOT NULL CHECK(length(device)=16), session BLOB NOT NULL CHECK(length(session)=16), registered_request BLOB NOT NULL CHECK(length(registered_request)=16) REFERENCES registration_journal(request), registered_revision BLOB NOT NULL CHECK(length(registered_revision)=8), generation BLOB NOT NULL CHECK(length(generation)=8), admission_request BLOB NOT NULL UNIQUE CHECK(length(admission_request)=16) REFERENCES admission_journal(request), admission_revision BLOB NOT NULL UNIQUE CHECK(length(admission_revision)=8), PRIMARY KEY(campaign,branch)) STRICT;\n";
type MetaColumns = (
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
    pub registrations: registration_replay::State,
    pub admission: Option<Meta>,
    pub version: i32,
}
pub(super) fn full_schema() -> String {
    format!("{}{}", registration_schema::SCHEMA, LEASE_SCHEMA)
}

pub(super) fn verify(
    connection: &Connection,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
) -> Result<Profile> {
    admission.validate(registration)?;
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 1 {
        return Ok(Profile {
            registrations: registration_replay::verify(connection, registration)?,
            admission: None,
            version,
        });
    }
    if version != 2 {
        return Err(crate::registration::RegistrationError::UnsupportedProfile.into());
    }
    registration_schema::verify_shape(connection, 2, &full_schema())?;
    from_validated_shape(connection, registration, admission, version)
}

/// Private metadata seam: caller already verified its complete selected table shape.
/// Admission metadata still binds the unchanged original seven-table schema hash.
pub(super) fn from_validated_shape(
    connection: &Connection,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
    version: i32,
) -> Result<Profile> {
    let registration_meta = registration_schema::read_meta(connection, registration)?;
    let registrations =
        registration_replay::from_validated_meta(connection, registration, registration_meta)?;
    let (universe, history, revision, head, policy_digest, schema_digest, policy_body, registration_digest): MetaColumns =
        connection.query_row("SELECT universe,history,revision,head,policy_digest,schema_digest,policy_body,registration_policy_digest FROM admission_meta WHERE singleton=1", [], |row| {
            Ok((bounded_blob(row, 0, 16)?, bounded_blob(row, 1, 16)?, bounded_blob(row, 2, 8)?,
                bounded_blob(row, 3, 32)?, bounded_blob(row, 4, 32)?, bounded_blob(row, 5, 32)?,
                bounded_blob(row, 6, 60)?, bounded_blob(row, 7, 32)?))
        })?;
    if universe != registration.scope.universe.as_bytes()
        || history != registration.scope.history.as_bytes()
        || policy_digest != admission.digest()
        || schema_digest != hash(full_schema().as_bytes())
        || policy_body != codec::policy_bytes(admission)
        || registration_digest != registration.digest()?
    {
        return Err(LeaseError::Corrupt);
    }
    Ok(Profile {
        registrations,
        admission: Some(Meta {
            revision: read_counter(&revision)?,
            head: codec::fixed(&head)?,
        }),
        version,
    })
}

/// Called only for a prepared first grant inside its authenticated immediate transaction.
/// The registration binding set is fixed before this profile is activated.
pub(super) fn activate(
    tx: &Transaction<'_>,
    registration: &RegistrationPolicy,
    admission: &AdmissionPolicy,
) -> Result<()> {
    let registration_digest = registration.digest()?;
    let policy_body = codec::policy_bytes(admission);
    if policy_body.len() != 60 {
        return Err(LeaseError::Corrupt);
    }
    let policy_digest = admission.digest();
    let schema_digest = hash(full_schema().as_bytes());
    tx.execute_batch(LEASE_SCHEMA)?;
    tx.execute(
        "INSERT INTO admission_meta VALUES(1,?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            registration.scope.universe.as_bytes(),
            registration.scope.history.as_bytes(),
            counter(0),
            [0u8; 32],
            policy_digest,
            schema_digest,
            policy_body,
            registration_digest
        ],
    )?;
    tx.pragma_update(None, "user_version", 2)?;
    Ok(())
}
