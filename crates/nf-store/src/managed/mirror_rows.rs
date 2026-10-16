use super::{
    error::Result,
    rows::{self, Cell, Key, Patch, Row},
};
use crate::model::{RequestStatus, State};
use std::collections::BTreeMap;
pub(super) fn b(value: impl AsRef<[u8]>) -> Cell {
    Cell::Blob(value.as_ref().to_vec())
}
pub(super) fn u(value: u64) -> Cell {
    b(value.to_be_bytes())
}
pub(super) fn i(value: i64) -> Cell {
    Cell::Integer(value)
}
pub(super) fn all(state: &State) -> Result<BTreeMap<String, BTreeMap<Key, Row>>> {
    let mut result = BTreeMap::new();
    let spec = state.world.to_spec();
    let aggregate = spec
        .revisions
        .iter()
        .map(|(id, revision)| vec![b(id.as_bytes()), u(revision.0)])
        .collect();
    let modules = spec
        .providers
        .iter()
        .map(|p| vec![b(p.id.as_bytes()), u(p.draws), u(p.cooldown_until.0)])
        .collect();
    let mut requests = Vec::new();
    for record in state.requests.values() {
        let (phase, sequence, rejection) = match record.status {
            RequestStatus::Pending { .. } => (1, Cell::Null, Cell::Null),
            RequestStatus::Committed {
                sequence,
                rejection,
                ..
            } => (
                2,
                u(sequence.0),
                i(i64::from(crate::codec::rejection_code(rejection))),
            ),
        };
        requests.push(vec![
            b(record.intent.request.as_bytes()),
            b(crate::model::binding(&record.intent, record.device)?.digest()),
            b(nf_kernel::intent_digest(&record.intent)?),
            b(record.intent.operation.as_bytes()),
            i(phase),
            sequence,
            rejection,
        ]);
    }
    let reservations = state
        .reservations
        .values()
        .map(|r| {
            vec![
                b(r.operation.as_bytes()),
                b(r.market.as_bytes()),
                u(r.amount),
            ]
        })
        .collect();
    let outbox = state
        .outbox
        .values()
        .map(|r| {
            vec![
                b(r.operation.as_bytes()),
                u(r.sequence.0),
                b(r.batch_digest),
                i(i64::from(crate::codec::rejection_code(r.rejection))),
            ]
        })
        .collect();
    for (name, values) in [
        ("aggregate_state", aggregate),
        ("module_state", modules),
        ("request_outcomes", requests),
        ("reservations", reservations),
        ("outbox", outbox),
    ] {
        let table = rows::table(name)?;
        let mut map = BTreeMap::new();
        for row in values {
            map.insert(rows::key(&row, table)?, row);
        }
        result.insert(name.to_string(), map);
    }
    Ok(result)
}
pub(super) fn diff(before: &State, after: &State) -> Result<BTreeMap<String, Patch>> {
    let old = all(before)?;
    let new = all(after)?;
    let mut result = BTreeMap::new();
    for (name, values) in old {
        let next = &new[&name];
        let table = rows::table(&name)?;
        let mut patch = Patch::default();
        for (key, row) in &values {
            if next.get(key) != Some(row) {
                patch.put(table, Some(row.clone()), next.get(key).cloned())?;
            }
        }
        for (key, row) in next {
            if !values.contains_key(key) {
                patch.put(table, None, Some(row.clone()))?;
            }
        }
        if !patch.0.is_empty() {
            result.insert(name, patch);
        }
    }
    Ok(result)
}
