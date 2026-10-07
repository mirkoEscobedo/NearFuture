use super::{codec, error::Result, model::*, state::State};
use crate::{
    StoreError,
    persistence::bounded_blob,
    schema::{hash, read_counter},
};
use rusqlite::Connection;
pub(crate) fn recover(
    c: &Connection,
    known: MiniatureKnownFrontiers,
) -> Result<(State, u64, [u8; 32])> {
    let (revision, bytes, digest, head) = c.query_row(
        "SELECT revision,state,digest,head FROM history_meta WHERE singleton=1",
        [],
        |r| {
            Ok((
                bounded_blob(r, 0, 8)?,
                bounded_blob(r, 1, 1_048_576)?,
                bounded_blob(r, 2, 32)?,
                bounded_blob(r, 3, 32)?,
            ))
        },
    )?;
    if hash(&bytes) != digest.as_slice() {
        return Err(StoreError::Corrupt.into());
    }
    let state = codec::decode(&bytes)?;
    let revision = read_counter(&revision)?;
    let head: [u8; 32] = head.try_into().map_err(|_| StoreError::Corrupt)?;
    let (universe, history, schema_digest, event) = c.query_row(
        "SELECT universe,history,schema_digest,event_seq FROM history_meta WHERE singleton=1",
        [],
        |r| {
            Ok((
                bounded_blob(r, 0, 16)?,
                bounded_blob(r, 1, 16)?,
                bounded_blob(r, 2, 32)?,
                bounded_blob(r, 3, 8)?,
            ))
        },
    )?;
    if schema_digest != super::schema::digest() {
        return Err(StoreError::UnsupportedSchema.into());
    }
    if state.scope() != known.storage.scope
        || universe != state.scope().universe.as_bytes()
        || history != state.scope().history.as_bytes()
    {
        return Err(StoreError::Scope.into());
    }
    if read_counter(&event)? != state.world.metadata().event_sequence.0 {
        return Err(StoreError::Corrupt.into());
    }
    if revision < known.storage.store_revision
        || state.world.metadata().event_sequence < known.storage.event_sequence
        || state
            .authority
            .map_or(nf_contract::identity::AuthorityTerm(0), |a| a.term)
            < known.minimum_authority_term
    {
        return Err(StoreError::StaleBackup.into());
    }
    let membership = crate::identity::load(c, state.scope())?.ok_or(StoreError::Corrupt)?;
    crate::identity::verify_minimum(c, known.storage)?;
    if state
        .authority
        .is_some_and(|a| a.membership_revision > membership.revision)
        || state
            .pending
            .as_ref()
            .is_some_and(|f| f.membership_revision() > membership.revision)
    {
        return Err(StoreError::Corrupt.into());
    }
    let mut checkpoint = stored(c.query_row(
        "SELECT revision,state,digest,head FROM snapshots ORDER BY revision DESC LIMIT 1",
        [],
        columns,
    )?)?;
    if checkpoint.0 > revision || checkpoint.1.scope() != state.scope() {
        return Err(StoreError::Corrupt.into());
    }
    let snapshot_sequence = checkpoint.1.world.metadata().event_sequence;
    let mut expected_events = Vec::new();
    let mut tail_count = 0;
    let mut statement=c.prepare("SELECT revision,kind,previous,digest,head,action,state FROM journal WHERE revision>?1 ORDER BY revision LIMIT 257")?;
    let mut rows = statement.query(rusqlite::params![crate::schema::counter(checkpoint.0)])?;
    while let Some(row) = rows.next()? {
        tail_count += 1;
        if tail_count > 256 {
            return Err(StoreError::Limit.into());
        }
        let rev = read_counter(&bounded_blob(row, 0, 8)?)?;
        let kind: u32 = row.get(1)?;
        let previous = bounded_blob(row, 2, 32)?;
        let digest: [u8; 32] = bounded_blob(row, 3, 32)?
            .try_into()
            .map_err(|_| StoreError::Corrupt)?;
        let new_head: [u8; 32] = bounded_blob(row, 4, 32)?
            .try_into()
            .map_err(|_| StoreError::Corrupt)?;
        let action = bounded_blob(row, 5, 1_048_576)?;
        let body = bounded_blob(row, 6, 1_048_576)?;
        if checkpoint.0.checked_add(1) != Some(rev)
            || previous != checkpoint.2
            || hash(&body) != digest
            || crate::persistence::head(checkpoint.3, rev, kind, &action, digest) != new_head
        {
            return Err(StoreError::Corrupt.into());
        }
        let candidate = codec::decode(&body)?;
        let expected = super::transitions::replay(&checkpoint.1, kind, &action)?;
        if expected != candidate {
            return Err(StoreError::Corrupt.into());
        }
        if kind == 2 {
            expected_events.push((candidate.world.metadata().event_sequence, action));
        }
        checkpoint = (rev, candidate, digest, new_head);
    }
    if checkpoint.0 != revision
        || checkpoint.1 != state
        || checkpoint.2 != hash(&bytes)
        || checkpoint.3 != head
    {
        return Err(StoreError::Corrupt.into());
    }
    let count: i64 = c.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM journal LIMIT 257)",
        [],
        |r| r.get(0),
    )?;
    let snapshots: i64 = c.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM snapshots LIMIT 2)",
        [],
        |r| r.get(0),
    )?;
    if count != tail_count || snapshots != 1 {
        return Err(StoreError::Corrupt.into());
    }
    let mut statement = c.prepare(
        "SELECT sequence,body,digest FROM events WHERE sequence>?1 ORDER BY sequence LIMIT 257",
    )?;
    let mut rows = statement.query(rusqlite::params![crate::schema::counter(
        snapshot_sequence.0
    )])?;
    let expected_count = expected_events.len();
    for (seq, body) in expected_events {
        let row = rows.next()?.ok_or(StoreError::Corrupt)?;
        if read_counter(&bounded_blob(row, 0, 8)?)? != seq.0
            || bounded_blob(row, 1, 1_048_576)? != body
            || bounded_blob(row, 2, 32)? != hash(&body)
        {
            return Err(StoreError::Corrupt.into());
        }
    }
    if rows.next()?.is_some() {
        return Err(StoreError::Corrupt.into());
    }
    let events: i64 = c.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM events LIMIT 257)",
        [],
        |r| r.get(0),
    )?;
    if events != expected_count as i64 {
        return Err(StoreError::Corrupt.into());
    }
    super::mirrors::verify(c, &state)?;
    Ok((state, revision, head))
}
type StoredColumns = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
fn columns(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredColumns> {
    Ok((
        bounded_blob(row, 0, 8)?,
        bounded_blob(row, 1, 1_048_576)?,
        bounded_blob(row, 2, 32)?,
        bounded_blob(row, 3, 32)?,
    ))
}
fn stored(
    (revision, bytes, digest, head): StoredColumns,
) -> Result<(u64, State, [u8; 32], [u8; 32])> {
    let digest: [u8; 32] = digest.try_into().map_err(|_| StoreError::Corrupt)?;
    if hash(&bytes) != digest {
        return Err(StoreError::Corrupt.into());
    }
    Ok((
        read_counter(&revision)?,
        codec::decode(&bytes)?,
        digest,
        head.try_into().map_err(|_| StoreError::Corrupt)?,
    ))
}
