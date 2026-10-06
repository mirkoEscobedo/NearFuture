use crate::{
    codec,
    error::{Result, StoreError},
    model::*,
    persistence::bounded_blob,
    schema::{counter, read_counter},
};
use rusqlite::{Connection, params};
type RequestColumns = (Vec<u8>, Vec<u8>, Vec<u8>, u32, Option<Vec<u8>>, Option<u32>);
fn count(connection: &Connection, table: &str, expected: usize, maximum: usize) -> Result<()> {
    let sql = format!(
        "SELECT count(*) FROM (SELECT 1 FROM {table} LIMIT {})",
        maximum + 1
    );
    let count: i64 = connection.query_row(&sql, [], |row| row.get(0))?;
    if count != expected as i64 {
        return Err(StoreError::Corrupt);
    }
    Ok(())
}
pub(crate) fn verify(connection: &Connection, state: &State) -> Result<()> {
    let spec = state.world.to_spec();
    count(connection, "aggregate_state", spec.revisions.len(), 160)?;
    count(connection, "module_state", spec.providers.len(), 32)?;
    count(
        connection,
        "request_outcomes",
        state.requests.len(),
        MAX_REQUESTS,
    )?;
    count(
        connection,
        "reservations",
        state.reservations.len(),
        MAX_RESERVATIONS,
    )?;
    count(connection, "outbox", state.outbox.len(), MAX_OUTBOX)?;
    for (id, revision) in spec.revisions {
        let bytes = connection.query_row(
            "SELECT revision FROM aggregate_state WHERE aggregate_id=?1",
            params![id.as_bytes()],
            |row| bounded_blob(row, 0, 8),
        )?;
        if read_counter(&bytes)? != revision.0 {
            return Err(StoreError::Corrupt);
        }
    }
    for value in spec.providers {
        let (draws, cooldown) = connection.query_row(
            "SELECT draws,cooldown FROM module_state WHERE provider_id=?1",
            params![value.id.as_bytes()],
            |row| Ok((bounded_blob(row, 0, 8)?, bounded_blob(row, 1, 8)?)),
        )?;
        if draws != counter(value.draws) || cooldown != counter(value.cooldown_until.0) {
            return Err(StoreError::Corrupt);
        }
    }
    for record in state.requests.values() {
        let(binding_digest,intent_digest,operation,phase,event,rejection):RequestColumns=connection.query_row("SELECT binding_digest,intent_digest,operation_id,phase,event_seq,rejection FROM request_outcomes WHERE request_id=?1",params![record.intent.request.as_bytes()],|row|Ok((bounded_blob(row,0,32)?,bounded_blob(row,1,32)?,bounded_blob(row,2,16)?,row.get(3)?,row.get(4)?,row.get(5)?)))?;
        let expected = match record.status {
            RequestStatus::Pending { .. } => (1, None, None),
            RequestStatus::Committed {
                sequence,
                rejection,
                ..
            } => (
                2,
                Some(counter(sequence.0).to_vec()),
                Some(codec::rejection_code(rejection)),
            ),
        };
        if binding_digest != binding(&record.intent, record.device)?.digest()
            || intent_digest != nf_kernel::intent_digest(&record.intent)?
            || operation != record.intent.operation.as_bytes()
            || (phase, event, rejection) != expected
        {
            return Err(StoreError::Corrupt);
        }
    }
    for value in state.reservations.values() {
        let (market, amount) = connection.query_row(
            "SELECT market_id,amount FROM reservations WHERE operation_id=?1",
            params![value.operation.as_bytes()],
            |row| Ok((bounded_blob(row, 0, 16)?, bounded_blob(row, 1, 8)?)),
        )?;
        if market != value.market.as_bytes() || amount != counter(value.amount) {
            return Err(StoreError::Corrupt);
        }
    }
    for value in state.outbox.values() {
        let (sequence, digest, rejection): (Vec<u8>, Vec<u8>, u32) = connection.query_row(
            "SELECT sequence,batch_digest,rejection FROM outbox WHERE operation_id=?1",
            params![value.operation.as_bytes()],
            |row| {
                Ok((
                    bounded_blob(row, 0, 8)?,
                    bounded_blob(row, 1, 32)?,
                    row.get(2)?,
                ))
            },
        )?;
        if sequence != counter(value.sequence.0)
            || digest != value.batch_digest
            || rejection != codec::rejection_code(value.rejection)
        {
            return Err(StoreError::Corrupt);
        }
    }
    Ok(())
}
