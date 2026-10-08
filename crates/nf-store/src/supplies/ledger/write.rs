use super::*;
use crate::schema::counter;
use nf_kernel::supplies::origin_bytes;
use rusqlite::params;

pub(in crate::supplies) fn apply(
    connection: &Connection,
    value: Operation,
    state: &State,
) -> Result<Entry> {
    apply_record(connection, JournalRecord::Supplies(value), state)
}
pub(crate) fn reserve_offer(
    connection: &Connection,
    value: nf_kernel::trade::ReserveOffer,
    state: &State,
) -> Result<nf_kernel::trade::TradeRequestOutcome> {
    let entry = apply_record(connection, JournalRecord::ReserveOffer(value), state)?;
    outcome::trade_decision(entry)
}
pub(crate) fn cancel_offer(
    connection: &Connection,
    value: nf_kernel::trade::CancelOffer,
    state: &State,
) -> Result<nf_kernel::trade::TradeReceipt> {
    let entry = apply_record(connection, JournalRecord::CancelOffer(value), state)?;
    outcome::cancel_receipt(entry)
}
pub(crate) fn accept_offer(
    connection: &Connection,
    value: nf_kernel::trade::AcceptOffer,
    state: &State,
) -> Result<nf_kernel::trade::TradeReceipt> {
    let entry = apply_record(connection, JournalRecord::AcceptOffer(value), state)?;
    outcome::accept_receipt(entry)
}
fn apply_record(connection: &Connection, value: JournalRecord, state: &State) -> Result<Entry> {
    let binding = value.binding();
    if let Some((stored, operation)) = state.requests.get(&value.request()) {
        if *stored != binding || *operation != value.id() {
            return Err(SuppliesRejection::Conflict.into());
        }
        return state
            .operations
            .get(operation)
            .copied()
            .ok_or(SuppliesStoreError::Corrupt);
    }
    if state.requests.len() as u64 >= CAPACITY {
        return Err(crate::StoreError::Backpressure.into());
    }
    let entry = if let Some(original) = state.operations.get(&value.id()) {
        if !original.operation.same_kind(value) || original.operation.economic() != value.economic()
        {
            return Err(SuppliesRejection::Conflict.into());
        }
        *original
    } else {
        if state.operations.len() as u64 >= CAPACITY {
            return Err(crate::StoreError::Backpressure.into());
        }
        if let JournalRecord::ReserveOffer(offer) = value
            && state.offers.contains_key(&offer.terms.offer)
        {
            return Err(SuppliesRejection::Conflict.into());
        }
        let decision = value.decision(&state.balances);
        let revision = state
            .revision
            .checked_add(1)
            .ok_or(crate::StoreError::Limit)?;
        let effects = value.effect(decision, state)?;
        let new_keys = effects
            .iter()
            .filter(|(key, _)| !state.balances.contains_key(key))
            .count();
        state
            .balances
            .len()
            .checked_add(new_keys)
            .filter(|count| *count <= CAPACITY as usize)
            .ok_or(crate::StoreError::Backpressure)?;
        connection.execute(
            "INSERT INTO supplies_operations VALUES(?1,?2,?3,?4,?5)",
            params![
                value.id().as_bytes(),
                value.economic(),
                value.bytes(),
                counter(revision),
                decision as i64
            ],
        )?;
        for (key, balance) in effects.iter() {
            connection.execute("INSERT INTO supplies_balances VALUES(?1,?2,?3,?4) ON CONFLICT(account,content,origin) DO UPDATE SET body=excluded.body",
                params![key.0.as_bytes(), key.1, origin_bytes(key.2), balance_bytes(*balance)])?;
        }
        flow_cache::write(connection, &effects)?;
        connection.execute(
            "UPDATE supplies_meta SET revision=?1 WHERE singleton=1",
            params![counter(revision)],
        )?;
        Entry {
            operation: value,
            revision,
            decision,
        }
    };
    connection.execute(
        "INSERT INTO supplies_requests VALUES(?1,?2,?3,?4)",
        params![
            value.request().as_bytes(),
            binding,
            value.id().as_bytes(),
            value.bytes()
        ],
    )?;
    Ok(entry)
}
