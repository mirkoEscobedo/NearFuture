use crate::{
    codec,
    error::Result,
    model::*,
    schema::{self, counter, hash},
};
use rusqlite::{Connection, Transaction, params};
pub(crate) fn mirror(connection: &Connection, state: &State) -> Result<()> {
    connection.execute_batch("DELETE FROM aggregate_state; DELETE FROM module_state; DELETE FROM request_outcomes; DELETE FROM reservations; DELETE FROM outbox;")?;
    let spec = state.world.to_spec();
    for (id, revision) in &spec.revisions {
        connection.execute(
            "INSERT INTO aggregate_state VALUES (?1,?2)",
            params![id.as_bytes(), counter(revision.0)],
        )?;
    }
    for value in &spec.providers {
        connection.execute(
            "INSERT INTO module_state VALUES (?1,?2,?3)",
            params![
                value.id.as_bytes(),
                counter(value.draws),
                counter(value.cooldown_until.0)
            ],
        )?;
    }
    for record in state.requests.values() {
        let (phase, sequence, rejection) = match record.status {
            RequestStatus::Pending { .. } => (1, None, None),
            RequestStatus::Committed {
                sequence,
                rejection,
                ..
            } => (
                2,
                Some(counter(sequence.0)),
                Some(codec::rejection_code(rejection)),
            ),
        };
        connection.execute(
            "INSERT INTO request_outcomes VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                record.intent.request.as_bytes(),
                binding(&record.intent, record.device)?.digest(),
                nf_kernel::intent_digest(&record.intent)?,
                record.intent.operation.as_bytes(),
                phase,
                sequence,
                rejection
            ],
        )?;
    }
    for value in state.reservations.values() {
        connection.execute(
            "INSERT INTO reservations VALUES (?1,?2,?3)",
            params![
                value.operation.as_bytes(),
                value.market.as_bytes(),
                counter(value.amount)
            ],
        )?;
    }
    for value in state.outbox.values() {
        connection.execute(
            "INSERT INTO outbox VALUES (?1,?2,?3,?4)",
            params![
                value.operation.as_bytes(),
                counter(value.sequence.0),
                value.batch_digest,
                codec::rejection_code(value.rejection)
            ],
        )?;
    }
    Ok(())
}
pub(crate) fn initialize(connection: &mut Connection, state: &State) -> Result<()> {
    let bytes = codec::encode(state)?;
    let digest = hash(&bytes);
    let scope = state.scope();
    let transaction = connection.transaction()?;
    transaction.execute_batch(schema::SCHEMA)?;
    transaction.pragma_update(None, "application_id", schema::APPLICATION_ID)?;
    transaction.pragma_update(None, "user_version", schema::SCHEMA_VERSION)?;
    transaction.execute(
        "INSERT INTO history_meta VALUES (1,?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            scope.universe.as_bytes(),
            scope.history.as_bytes(),
            counter(0),
            counter(state.world.view().event_seq().0),
            bytes,
            digest,
            [0u8; 32],
            schema::schema_digest()
        ],
    )?;
    transaction.execute(
        "INSERT INTO snapshots VALUES (?1,?2,?3,?4)",
        params![counter(0), bytes, digest, [0u8; 32]],
    )?;
    mirror(&transaction, state)?;
    transaction.commit()?;
    Ok(())
}
pub(crate) fn head(
    previous: [u8; 32],
    revision: u64,
    kind: u32,
    action: &[u8],
    digest: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"NF-STORE-JOURNAL-1\0".to_vec();
    bytes.extend_from_slice(&previous);
    bytes.extend_from_slice(&counter(revision));
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&hash(action));
    bytes.extend_from_slice(&digest);
    hash(&bytes)
}
pub(crate) fn write(
    transaction: &Transaction<'_>,
    old: &State,
    next: &State,
    revision: u64,
    previous_head: [u8; 32],
    kind: u32,
    action: &[u8],
) -> Result<[u8; 32]> {
    let bytes = codec::encode(next)?;
    let digest = hash(&bytes);
    let previous = hash(&codec::encode(old)?);
    let new_head = head(previous_head, revision, kind, action, digest);
    transaction.execute(
        "INSERT INTO journal VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            counter(revision),
            kind,
            previous,
            digest,
            new_head,
            action,
            bytes
        ],
    )?;
    if kind == 2 {
        let batch = nf_kernel::decode_batch(action)?;
        transaction.execute(
            "INSERT INTO events VALUES (?1,?2,?3)",
            params![counter(batch.sequence.0), action, hash(action)],
        )?;
    }
    transaction.execute("UPDATE history_meta SET revision=?1,event_seq=?2,state=?3,digest=?4,head=?5 WHERE singleton=1",params![counter(revision),counter(next.world.view().event_seq().0),bytes,digest,new_head])?;
    mirror(transaction, next)?;
    Ok(new_head)
}
pub(crate) fn bounded_blob(
    row: &rusqlite::Row<'_>,
    index: usize,
    maximum: usize,
) -> rusqlite::Result<Vec<u8>> {
    let bytes = row.get_ref(index)?.as_blob()?;
    if bytes.len() > maximum {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(bytes.to_vec())
}
