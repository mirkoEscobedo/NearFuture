use super::{
    codec,
    model::{OutboxProfile, OutgoingEntry, OutgoingState},
    receipt,
};
use crate::chat::{ChatStoreError, Result, SignedMessage, codec as chat_codec};
use nf_contract::identity::RequestId;
use rusqlite::{Connection, params};
use std::collections::{BTreeMap, BTreeSet};
const APPLICATION_ID: i32 = 0x4e46_434f;
const VERSION: i32 = 1;
// Delivered projection requires a bounded canonical receipt and the retained receiver signature.
// This profile remains unpublished while the combined Pending/Delivered controls are completed.
const SCHEMA: &str = "CREATE TABLE outbox_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), profile_body BLOB NOT NULL CHECK(length(profile_body)<=512), profile_digest BLOB NOT NULL CHECK(length(profile_digest)=32), schema_digest BLOB NOT NULL CHECK(length(schema_digest)=32), revision BLOB NOT NULL CHECK(length(revision)=8)) STRICT;\nCREATE TABLE outbox_messages (message BLOB PRIMARY KEY CHECK(length(message)=16), account BLOB NOT NULL CHECK(length(account)=16), device BLOB NOT NULL CHECK(length(device)=16), sequence BLOB NOT NULL CHECK(length(sequence)=8), original_request BLOB NOT NULL UNIQUE CHECK(length(original_request)=16), body BLOB NOT NULL CHECK(length(body)<=2157), signature BLOB NOT NULL CHECK(length(signature)=64), phase INTEGER NOT NULL CHECK(phase IN (1,2)), receipt BLOB CHECK(receipt IS NULL OR length(receipt)<=420), CHECK((phase=1 AND receipt IS NULL) OR (phase=2 AND receipt IS NOT NULL)), UNIQUE(account,device,sequence)) STRICT;\n";
pub(super) struct State {
    pub revision: u64,
    pub entries: BTreeMap<[u8; 16], OutgoingEntry>,
}
pub(super) fn initialize(connection: &mut Connection, profile: &OutboxProfile) -> Result<()> {
    let tx = connection.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    let body = codec::profile_bytes(profile);
    tx.execute(
        "INSERT INTO outbox_meta VALUES(1,?1,?2,?3,?4)",
        params![
            body,
            chat_codec::hash(&body),
            chat_codec::hash(SCHEMA.as_bytes()),
            crate::schema::counter(0)
        ],
    )?;
    tx.commit()?;
    Ok(())
}
pub(super) fn verify(connection: &Connection, profile: &OutboxProfile) -> Result<State> {
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
    let expected: BTreeMap<_, _> = SCHEMA
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
    let mut statement = connection.prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 3")?;
    let actual: BTreeMap<String, String> = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if extra || actual != expected {
        return Err(ChatStoreError::UnsupportedProfile);
    }
    let meta_count: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM outbox_meta LIMIT 2)",
        [],
        |r| r.get(0),
    )?;
    if meta_count != 1 {
        return Err(ChatStoreError::Corrupt);
    }
    let (body, digest, schema_digest, revision): (Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>) = connection.query_row("SELECT profile_body,profile_digest,schema_digest,revision FROM outbox_meta WHERE singleton=1",[],|r|Ok((crate::persistence::bounded_blob(r,0,512)?,crate::persistence::bounded_blob(r,1,32)?,crate::persistence::bounded_blob(r,2,32)?,crate::persistence::bounded_blob(r,3,8)?)))?;
    if body != codec::profile_bytes(profile) {
        return Err(ChatStoreError::Policy);
    }
    if digest != chat_codec::hash(&body) || schema_digest != chat_codec::hash(SCHEMA.as_bytes()) {
        return Err(ChatStoreError::Corrupt);
    }
    let revision = crate::schema::read_counter(&revision)?;
    let mut state = State {
        revision,
        entries: BTreeMap::new(),
    };
    let mut requests = BTreeSet::new();
    let mut positions = BTreeSet::new();
    let mut delivered = 0u64;
    let mut statement = connection.prepare("SELECT message,account,device,sequence,original_request,body,signature,phase,receipt FROM outbox_messages ORDER BY message LIMIT 4097")?;
    let rows = statement.query_map([], |r| {
        Ok((
            crate::persistence::bounded_blob(r, 0, 16)?,
            crate::persistence::bounded_blob(r, 1, 16)?,
            crate::persistence::bounded_blob(r, 2, 16)?,
            crate::persistence::bounded_blob(r, 3, 8)?,
            crate::persistence::bounded_blob(r, 4, 16)?,
            crate::persistence::bounded_blob(r, 5, 2157)?,
            crate::persistence::bounded_blob(r, 6, 64)?,
            r.get::<_, i64>(7)?,
            match r.get_ref(8)? {
                rusqlite::types::ValueRef::Null => None,
                _ => Some(crate::persistence::bounded_blob(r, 8, 420)?),
            },
        ))
    })?;
    for row in rows {
        if state.entries.len() >= 4096 {
            return Err(ChatStoreError::Corrupt);
        }
        let (id, account, device, sequence, request, body, signature, phase, receipt) = row?;
        let signed = SignedMessage {
            message: chat_codec::decode_message(&body).map_err(|_| ChatStoreError::Corrupt)?,
            signature: signature
                .as_slice()
                .try_into()
                .map_err(|_| ChatStoreError::Corrupt)?,
        };
        let request = RequestId::from_bytes(
            request
                .as_slice()
                .try_into()
                .map_err(|_| ChatStoreError::Corrupt)?,
        );
        codec::validate(profile, request, &signed).map_err(|_| ChatStoreError::Corrupt)?;
        if id.as_slice() != signed.message.message
            || account.as_slice() != signed.message.author.account.as_bytes()
            || device.as_slice() != signed.message.author.device.as_bytes()
            || crate::schema::read_counter(&sequence)? != signed.message.sequence
            || !requests.insert(request)
            || !positions.insert(signed.message.sequence)
        {
            return Err(ChatStoreError::Corrupt);
        }
        let id = signed.message.message;
        let mut entry = OutgoingEntry {
            original_request: request,
            signed,
            state: OutgoingState::Pending,
        };
        match (phase, receipt) {
            (1, None) => {}
            (2, Some(encoded)) => {
                let signed = receipt::decode(&encoded).map_err(|_| ChatStoreError::Corrupt)?;
                receipt::validate(profile, &entry, &signed).map_err(|_| ChatStoreError::Corrupt)?;
                entry.state = OutgoingState::Delivered(Box::new(signed));
                delivered += 1;
            }
            _ => return Err(ChatStoreError::Corrupt),
        }
        if state.entries.insert(id, entry).is_some() {
            return Err(ChatStoreError::Corrupt);
        }
    }
    if revision > 8192 || revision != state.entries.len() as u64 + delivered {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(state)
}
