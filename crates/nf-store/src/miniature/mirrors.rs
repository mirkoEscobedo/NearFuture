use super::{codec, error::Result, model::*, state::State};
use crate::{
    StoreError,
    persistence::bounded_blob,
    schema::{counter, hash},
};
use nf_kernel::miniature::encode_miniature_intent;
use rusqlite::{Connection, params};
pub(crate) fn write(c: &Connection, state: &State) -> Result<()> {
    c.execute_batch("DELETE FROM aggregate_state; DELETE FROM module_state; DELETE FROM request_outcomes; DELETE FROM reservations; DELETE FROM outbox; DELETE FROM world_authority;")?;
    let m = state.world.metadata();
    for (id, revision) in [
        (m.aggregate, state.world.component().revision()),
        (m.provider_aggregate, m.provider_revision),
    ] {
        c.execute(
            "INSERT INTO aggregate_state VALUES (?1,?2)",
            params![id.as_bytes(), counter(revision.0)],
        )?;
    }
    c.execute(
        "INSERT INTO module_state VALUES (?1,?2,?3)",
        params![m.provider.as_bytes(), counter(0), counter(0)],
    )?;
    for r in state.requests.values() {
        let (phase, event, rejection) = columns(r.status);
        c.execute(
            "INSERT INTO request_outcomes VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                r.intent.request.as_bytes(),
                r.binding_digest,
                hash(&encode_miniature_intent(&r.intent)?),
                r.intent.operation.as_bytes(),
                phase,
                event,
                rejection
            ],
        )?;
    }
    if let Some(h) = state.pending.as_ref().and_then(|f| f.reservation()) {
        c.execute(
            "INSERT INTO reservations VALUES (?1,?2,?3,?4)",
            params![
                h.operation().as_bytes(),
                h.faction().as_bytes(),
                counter(h.credits()),
                counter(h.supplies())
            ],
        )?;
    }
    for o in state.outbox.values() {
        c.execute(
            "INSERT INTO outbox VALUES (?1,?2,?3,?4)",
            params![
                o.operation.as_bytes(),
                counter(o.sequence.0),
                o.batch_digest,
                codec::code(o.rejection)
            ],
        )?;
    }
    if let Some(a) = state.authority {
        c.execute(
            "INSERT INTO world_authority VALUES (1,?1,?2,?3,?4,?5)",
            params![
                counter(a.term.0),
                counter(a.session.0),
                a.account.as_bytes(),
                a.device.as_bytes(),
                counter(a.membership_revision)
            ],
        )?;
    }
    Ok(())
}
fn columns(status: MiniatureRequestStatus) -> (u8, Option<Vec<u8>>, Option<u8>) {
    match status {
        MiniatureRequestStatus::Pending { .. } => (1, None, None),
        MiniatureRequestStatus::Committed {
            sequence,
            rejection,
            ..
        } => (
            2,
            Some(counter(sequence.0).to_vec()),
            Some(codec::code(rejection)),
        ),
    }
}
fn count(c: &Connection, table: &str, expected: usize, max: usize) -> Result<()> {
    let sql = format!(
        "SELECT count(*) FROM (SELECT 1 FROM {table} LIMIT {})",
        max + 1
    );
    let n: i64 = c.query_row(&sql, [], |r| r.get(0))?;
    if n != expected as i64 {
        return Err(StoreError::Corrupt.into());
    }
    Ok(())
}
pub(crate) fn verify(c: &Connection, state: &State) -> Result<()> {
    for (table, expected, max) in [
        ("aggregate_state", 2, 2),
        ("module_state", 1, 1),
        ("request_outcomes", state.requests.len(), 4096),
        (
            "reservations",
            usize::from(
                state
                    .pending
                    .as_ref()
                    .and_then(|f| f.reservation())
                    .is_some(),
            ),
            1,
        ),
        ("outbox", state.outbox.len(), 256),
        ("world_authority", usize::from(state.authority.is_some()), 1),
    ] {
        count(c, table, expected, max)?;
    }
    let m = state.world.metadata();
    for (id, rev) in [
        (m.aggregate, state.world.component().revision()),
        (m.provider_aggregate, m.provider_revision),
    ] {
        let value = c.query_row(
            "SELECT revision FROM aggregate_state WHERE aggregate_id=?1",
            params![id.as_bytes()],
            |r| bounded_blob(r, 0, 8),
        )?;
        if value != counter(rev.0) {
            return Err(StoreError::Corrupt.into());
        }
    }
    let (draws, cooldown) = c.query_row(
        "SELECT draws,cooldown FROM module_state WHERE provider_id=?1",
        params![m.provider.as_bytes()],
        |r| Ok((bounded_blob(r, 0, 8)?, bounded_blob(r, 1, 8)?)),
    )?;
    if draws != counter(0) || cooldown != counter(0) {
        return Err(StoreError::Corrupt.into());
    }
    for r in state.requests.values() {
        let (binding,intent,operation,phase,event,rejection)=c.query_row("SELECT binding_digest,intent_digest,operation_id,phase,event_seq,rejection FROM request_outcomes WHERE request_id=?1",params![r.intent.request.as_bytes()],|r|Ok((bounded_blob(r,0,32)?,bounded_blob(r,1,32)?,bounded_blob(r,2,16)?,r.get::<_,u8>(3)?,if matches!(r.get_ref(4)?,rusqlite::types::ValueRef::Null){None}else{Some(bounded_blob(r,4,8)?)},r.get::<_,Option<u8>>(5)?)))?;
        if binding != r.binding_digest
            || intent != hash(&encode_miniature_intent(&r.intent)?)
            || operation != r.intent.operation.as_bytes()
            || (phase, event, rejection) != columns(r.status)
        {
            return Err(StoreError::Corrupt.into());
        }
    }
    if let Some(h) = state.pending.as_ref().and_then(|f| f.reservation()) {
        let (faction, credits, supplies) = c.query_row(
            "SELECT faction_id,credits,supplies FROM reservations WHERE operation_id=?1",
            params![h.operation().as_bytes()],
            |r| {
                Ok((
                    bounded_blob(r, 0, 16)?,
                    bounded_blob(r, 1, 8)?,
                    bounded_blob(r, 2, 8)?,
                ))
            },
        )?;
        if faction != h.faction().as_bytes()
            || credits != counter(h.credits())
            || supplies != counter(h.supplies())
        {
            return Err(StoreError::Corrupt.into());
        }
    }
    for o in state.outbox.values() {
        let (sequence, digest, rejection) = c.query_row(
            "SELECT sequence,batch_digest,rejection FROM outbox WHERE operation_id=?1",
            params![o.operation.as_bytes()],
            |r| {
                Ok((
                    bounded_blob(r, 0, 8)?,
                    bounded_blob(r, 1, 32)?,
                    r.get::<_, u8>(2)?,
                ))
            },
        )?;
        if sequence != counter(o.sequence.0)
            || digest != o.batch_digest
            || rejection != codec::code(o.rejection)
        {
            return Err(StoreError::Corrupt.into());
        }
    }
    if let Some(a) = state.authority {
        let (term,session,account,device,member)=c.query_row("SELECT term,session,account_id,device_id,membership_revision FROM world_authority WHERE singleton=1",[],|r|Ok((bounded_blob(r,0,8)?,bounded_blob(r,1,8)?,bounded_blob(r,2,16)?,bounded_blob(r,3,16)?,bounded_blob(r,4,8)?)))?;
        if term != counter(a.term.0)
            || session != counter(a.session.0)
            || account != a.account.as_bytes()
            || device != a.device.as_bytes()
            || member != counter(a.membership_revision)
        {
            return Err(StoreError::Corrupt.into());
        }
    }
    Ok(())
}
