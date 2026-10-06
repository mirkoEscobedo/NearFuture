use crate::{
    codec,
    error::{Result, StoreError},
    model::*,
    persistence::{self, bounded_blob},
    schema::{counter, hash, read_counter},
};
use nf_contract::identity::*;
use rusqlite::{Connection, params};
type StoredColumns = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
struct Stored {
    revision: u64,
    state: State,
    digest: [u8; 32],
    head: [u8; 32],
}
fn stored(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredColumns> {
    Ok((
        bounded_blob(row, 0, 8)?,
        bounded_blob(row, 1, MAX_STATE_BYTES)?,
        bounded_blob(row, 2, 32)?,
        bounded_blob(row, 3, 32)?,
    ))
}
fn decode(values: StoredColumns) -> Result<Stored> {
    let (revision, body, digest, head) = values;
    let digest: [u8; 32] = digest.try_into().map_err(|_| StoreError::Corrupt)?;
    if hash(&body) != digest {
        return Err(StoreError::Corrupt);
    }
    Ok(Stored {
        revision: read_counter(&revision)?,
        state: codec::decode(&body)?,
        digest,
        head: head.try_into().map_err(|_| StoreError::Corrupt)?,
    })
}
pub(crate) fn recover(
    connection: &Connection,
    known: KnownFrontiers,
) -> Result<(State, u64, [u8; 32])> {
    let current = decode(connection.query_row(
        "SELECT revision,state,digest,head FROM history_meta WHERE singleton=1",
        [],
        stored,
    )?)?;
    let (universe, history, schema_digest, event): StoredColumns = connection.query_row(
        "SELECT universe,history,schema_digest,event_seq FROM history_meta WHERE singleton=1",
        [],
        |row| {
            Ok((
                bounded_blob(row, 0, 16)?,
                bounded_blob(row, 1, 16)?,
                bounded_blob(row, 2, 32)?,
                bounded_blob(row, 3, 8)?,
            ))
        },
    )?;
    if schema_digest != crate::schema::schema_digest() {
        return Err(StoreError::UnsupportedSchema);
    }
    let scope = current.state.scope();
    if universe != scope.universe.as_bytes()
        || history != scope.history.as_bytes()
        || scope != known.scope
    {
        return Err(StoreError::Scope);
    }
    if read_counter(&event)? != current.state.world.view().event_seq().0 {
        return Err(StoreError::Corrupt);
    }
    if current.revision < known.store_revision
        || current.state.world.view().event_seq() < known.event_sequence
    {
        return Err(StoreError::StaleBackup);
    }
    let mut checkpoint = decode(connection.query_row(
        "SELECT revision,state,digest,head FROM snapshots ORDER BY revision DESC LIMIT 1",
        [],
        stored,
    )?)?;
    if checkpoint.revision > current.revision || checkpoint.state.scope() != scope {
        return Err(StoreError::Corrupt);
    }
    let snapshot_sequence = checkpoint.state.world.view().event_seq();
    let mut expected_events = Vec::new();
    let mut statement=connection.prepare("SELECT revision,kind,previous,digest,head,action,state FROM journal WHERE revision>?1 ORDER BY revision LIMIT 257")?;
    let mut rows = statement.query(params![counter(checkpoint.revision)])?;
    let mut tail_count = 0;
    while let Some(row) = rows.next()? {
        tail_count += 1;
        if tail_count > 256 {
            return Err(StoreError::Limit);
        }
        let revision = read_counter(&bounded_blob(row, 0, 8)?)?;
        if checkpoint.revision.checked_add(1) != Some(revision) {
            return Err(StoreError::Corrupt);
        }
        let kind: u32 = row.get(1)?;
        let previous = bounded_blob(row, 2, 32)?;
        let digest: [u8; 32] = bounded_blob(row, 3, 32)?
            .try_into()
            .map_err(|_| StoreError::Corrupt)?;
        let head: [u8; 32] = bounded_blob(row, 4, 32)?
            .try_into()
            .map_err(|_| StoreError::Corrupt)?;
        let action = bounded_blob(row, 5, MAX_STATE_BYTES)?;
        let body = bounded_blob(row, 6, MAX_STATE_BYTES)?;
        if previous != checkpoint.digest
            || hash(&body) != digest
            || persistence::head(checkpoint.head, revision, kind, &action, digest) != head
        {
            return Err(StoreError::Corrupt);
        }
        let candidate = codec::decode(&body)?;
        let expected = match kind {
            1 => {
                let frontier =
                    nf_kernel::decode_frontier(&action).map_err(|_| StoreError::Corrupt)?;
                let mut devices = Vec::new();
                for job in frontier.jobs() {
                    let record = candidate
                        .requests
                        .get(&job.intent().request)
                        .ok_or(StoreError::Corrupt)?;
                    devices.push(PrincipalDevice {
                        request: record.intent.request,
                        device: record.device,
                    });
                }
                crate::transitions::prepare(
                    &checkpoint.state,
                    &frontier,
                    &devices,
                    &candidate.reservations.values().copied().collect::<Vec<_>>(),
                )?
            }
            2 => {
                let batch = nf_kernel::decode_batch(&action).map_err(|_| StoreError::Corrupt)?;
                expected_events.push((batch.sequence, action.clone()));
                crate::transitions::commit(&checkpoint.state, &batch)?
            }
            3 => {
                let operation = OperationId::from_bytes(
                    action
                        .as_slice()
                        .try_into()
                        .map_err(|_| StoreError::Corrupt)?,
                );
                let mut next = checkpoint.state.clone();
                if next.outbox.remove(&operation).is_none() {
                    return Err(StoreError::Corrupt);
                }
                next
            }
            _ => return Err(StoreError::UnsupportedSchema),
        };
        if expected != candidate {
            return Err(StoreError::Corrupt);
        }
        checkpoint = Stored {
            revision,
            state: candidate,
            digest,
            head,
        };
    }
    if checkpoint.revision != current.revision
        || checkpoint.state != current.state
        || checkpoint.digest != current.digest
        || checkpoint.head != current.head
    {
        return Err(StoreError::Corrupt);
    }
    let mut statement = connection.prepare(
        "SELECT sequence,body,digest FROM events WHERE sequence>?1 ORDER BY sequence LIMIT 257",
    )?;
    let mut rows = statement.query(params![counter(snapshot_sequence.0)])?;
    for (sequence, body) in expected_events {
        let row = rows.next()?.ok_or(StoreError::Corrupt)?;
        if read_counter(&bounded_blob(row, 0, 8)?)? != sequence.0
            || bounded_blob(row, 1, MAX_STATE_BYTES)? != body
            || bounded_blob(row, 2, 32)? != hash(&body)
        {
            return Err(StoreError::Corrupt);
        }
    }
    if rows.next()?.is_some() {
        return Err(StoreError::Corrupt);
    }
    crate::mirrors::verify(connection, &current.state)?;
    crate::identity::verify_minimum(connection, known)?;
    Ok((current.state, current.revision, current.head))
}
