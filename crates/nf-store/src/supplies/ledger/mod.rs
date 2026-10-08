mod effect;
mod flow;
mod flow_cache;
mod journal;
mod outcome;
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
use effect::Effects;
use journal::JournalRecord;
pub(crate) use outcome::{offer_state, offer_status, outbox, status};
pub(super) const FLOW_SCHEMA: &str = flow_cache::SCHEMA;
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
    pub operation: JournalRecord,
    pub revision: u64,
    pub decision: Decision,
}
/// Retained accepted version; this first profile6 slice never reuses a claimed OfferId.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct OfferHistory {
    pub reservation: OperationId,
    pub finalization: Option<OperationId>,
}
#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::supplies) enum LedgerMode {
    Legacy,
    Accepting,
}
#[derive(Eq, PartialEq)]
pub(crate) struct State {
    pub(in crate::supplies) mode: LedgerMode,
    pub(in crate::supplies) flows: BTreeMap<Key, flow::Flow>,
    pub revision: u64,
    pub(in crate::supplies) operations: BTreeMap<OperationId, Entry>,
    pub(in crate::supplies) requests: BTreeMap<RequestId, ([u8; 32], OperationId)>,
    pub(in crate::supplies) balances: BTreeMap<Key, Balances>,
    pub(in crate::supplies) offers: BTreeMap<nf_kernel::trade::OfferId, OfferHistory>,
}
pub(super) use write::apply;
pub(crate) use write::{accept_offer, cancel_offer, reserve_offer};
impl State {
    pub(in crate::supplies) fn flow(&self, key: &Key) -> flow::Flow {
        self.flows.get(key).copied().unwrap_or_default()
    }
    pub(in crate::supplies) fn conservation(&self, key: &Key, stock: Balances) -> Result<()> {
        match self.mode {
            LedgerMode::Legacy => conservation(stock),
            LedgerMode::Accepting => flow::conservation(stock, self.flow(key)),
        }
    }
}
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
pub(crate) fn verify(
    connection: &Connection,
    policy: &SuppliesPolicy,
    trade: Option<&crate::trade::policy::SelectedPolicy>,
) -> Result<State> {
    replay::load(connection, policy, trade)
}
