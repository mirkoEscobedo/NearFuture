use super::*;
use crate::schema::counter;
use nf_kernel::supplies::origin_bytes;
use rusqlite::params;

pub(in crate::supplies) fn apply(
    connection: &Connection,
    value: Operation,
    state: &State,
) -> Result<Entry> {
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
        if original.operation.economic() != value.economic() {
            return Err(SuppliesRejection::Conflict.into());
        }
        *original
    } else {
        if state.operations.len() as u64 >= CAPACITY {
            return Err(crate::StoreError::Backpressure.into());
        }
        let key = value.key();
        let revision = state
            .revision
            .checked_add(1)
            .ok_or(crate::StoreError::Limit)?;
        let mut balance = state.balances.get(&key).copied().unwrap_or_default();
        let insufficient = matches!(value, Operation::Reserve(reserve) if reserve.amount > balance.available)
            || matches!(value, Operation::Burn(burn) if burn.amount > balance.available);
        let decision = if insufficient {
            Decision::InsufficientAvailable
        } else {
            Decision::Accepted
        };
        if decision == Decision::Accepted {
            if !state.balances.contains_key(&key) && state.balances.len() as u64 >= CAPACITY {
                return Err(crate::StoreError::Backpressure.into());
            }
            value.apply(&mut balance)?;
        }
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
        if decision == Decision::Accepted {
            connection.execute("INSERT INTO supplies_balances VALUES(?1,?2,?3,?4) ON CONFLICT(account,content,origin) DO UPDATE SET body=excluded.body",
                params![key.0.as_bytes(), key.1, origin_bytes(key.2), balance_bytes(balance)])?;
        }
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
