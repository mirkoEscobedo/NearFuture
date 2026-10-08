mod record;
mod replay;
mod write;
use super::error::{Result, SuppliesStoreError};
use nf_contract::identity::{AccountId, OperationId, RequestId};
use nf_kernel::supplies::{Balances, Issuance, Origin, SuppliesPolicy, SuppliesRejection};
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) const CAPACITY: u64 = 4096;
pub(super) type Key = (AccountId, [u8; 32], Origin);
pub(super) use record::Operation;
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Decision {
    Accepted = 0,
    InsufficientAvailable = 1,
}
impl Decision {
    pub fn decode(value: i64) -> Result<Self> {
        match value {
            0 => Ok(Self::Accepted),
            1 => Ok(Self::InsufficientAvailable),
            _ => Err(SuppliesStoreError::Corrupt),
        }
    }
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct Entry {
    pub operation: Operation,
    pub revision: u64,
    pub decision: Decision,
}
#[derive(Eq, PartialEq)]
pub(super) struct State {
    pub revision: u64,
    pub operations: BTreeMap<OperationId, Entry>,
    pub requests: BTreeMap<RequestId, ([u8; 32], OperationId)>,
    pub balances: BTreeMap<Key, Balances>,
}
pub(super) use write::apply;
pub(super) fn conservation(value: Balances) -> Result<()> {
    let remaining = value
        .available
        .checked_add(value.reserved)
        .and_then(|n| n.checked_add(value.pending))
        .and_then(|n| n.checked_add(value.externalized))
        .and_then(|n| n.checked_add(value.burned))
        .ok_or(SuppliesStoreError::Corrupt)?;
    if remaining != value.minted {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(())
}
pub(super) fn balance_bytes(value: Balances) -> [u8; 48] {
    let mut bytes = [0; 48];
    for (slot, value) in bytes.as_chunks_mut::<8>().0.iter_mut().zip([
        value.available,
        value.reserved,
        value.pending,
        value.externalized,
        value.minted,
        value.burned,
    ]) {
        slot.copy_from_slice(&value.to_be_bytes());
    }
    bytes
}
pub(super) fn mint(value: &mut Balances, amount: u64) -> Result<()> {
    let available = value
        .available
        .checked_add(amount)
        .ok_or(crate::StoreError::Limit)?;
    let minted = value
        .minted
        .checked_add(amount)
        .ok_or(crate::StoreError::Limit)?;
    let next = Balances {
        available,
        minted,
        ..*value
    };
    conservation(next)?;
    *value = next;
    Ok(())
}
pub(super) fn destroy(value: &mut Balances, amount: u64) -> Result<()> {
    let available = value
        .available
        .checked_sub(amount)
        .ok_or(SuppliesRejection::Limit)?;
    let burned = value
        .burned
        .checked_add(amount)
        .ok_or(crate::StoreError::Limit)?;
    let next = Balances {
        available,
        burned,
        ..*value
    };
    conservation(next)?;
    *value = next;
    Ok(())
}
pub(super) fn reserve_units(value: &mut Balances, amount: u64) -> Result<()> {
    let available = value
        .available
        .checked_sub(amount)
        .ok_or(SuppliesRejection::Limit)?;
    let reserved = value
        .reserved
        .checked_add(amount)
        .ok_or(crate::StoreError::Limit)?;
    let next = Balances {
        available,
        reserved,
        ..*value
    };
    conservation(next)?;
    *value = next;
    Ok(())
}
pub(super) fn capacity(connection: &Connection, table: &str) -> Result<()> {
    let count: u64 =
        connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
    if count > CAPACITY {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(())
}
pub(super) fn verify(connection: &Connection, policy: &SuppliesPolicy) -> Result<State> {
    replay::load(connection, policy)
}
