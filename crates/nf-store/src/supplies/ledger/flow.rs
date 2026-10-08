//! Pure accepting-profile arithmetic: coalesce every leg before any durable write.
use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::supplies) struct Flow {
    pub received: u64,
    pub sent: u64,
}
impl Flow {
    pub fn bytes(self) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&self.received.to_be_bytes());
        bytes[8..].copy_from_slice(&self.sent.to_be_bytes());
        bytes
    }
    pub fn nonzero(self) -> bool {
        self.received != 0 || self.sent != 0
    }
}
pub(in crate::supplies) struct Batch {
    pub stock: Vec<(Key, Balances)>,
    pub flows: Vec<(Key, Flow)>,
}
#[derive(Default)]
struct Delta {
    available_add: u64,
    available_sub: u64,
    reserved_add: u64,
    reserved_sub: u64,
    minted_add: u64,
    burned_add: u64,
    received_add: u64,
    sent_add: u64,
}
fn add(value: &mut u64, amount: u64) -> Result<()> {
    *value = value.checked_add(amount).ok_or(crate::StoreError::Limit)?;
    Ok(())
}
fn sum(stock: Balances) -> u128 {
    u128::from(stock.available)
        + u128::from(stock.reserved)
        + u128::from(stock.pending)
        + u128::from(stock.externalized)
        + u128::from(stock.burned)
}
pub(super) fn conservation(stock: Balances, flow: Flow) -> Result<()> {
    if sum(stock) + u128::from(flow.sent) != u128::from(stock.minted) + u128::from(flow.received) {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(())
}
/// Maps are bounded to4096 u64 rows, so all grouped totals fit u128 exactly.
pub(super) fn validate(
    balances: &BTreeMap<Key, Balances>,
    flows: &BTreeMap<Key, Flow>,
) -> Result<()> {
    if balances.len() > CAPACITY as usize
        || flows.len() > CAPACITY as usize
        || flows
            .iter()
            .any(|(key, flow)| !flow.nonzero() || !balances.contains_key(key))
    {
        return Err(SuppliesStoreError::Corrupt);
    }
    let mut groups: BTreeMap<([u8; 32], Origin), (u128, u128, u128, u128)> = BTreeMap::new();
    for (key, stock) in balances {
        let flow = flows.get(key).copied().unwrap_or_default();
        conservation(*stock, flow)?;
        let totals = groups.entry((key.1, key.2)).or_default();
        totals.0 += sum(*stock);
        totals.1 += u128::from(stock.minted);
        totals.2 += u128::from(flow.received);
        totals.3 += u128::from(flow.sent);
    }
    if groups
        .values()
        .any(|(stock, minted, received, sent)| stock != minted || received != sent)
    {
        return Err(SuppliesStoreError::Corrupt);
    }
    Ok(())
}
fn key(account: AccountId, asset: nf_kernel::trade::AssetTerms) -> Key {
    (account, asset.content, asset.origin)
}
fn lock(delta: &mut Delta, amount: u64) -> Result<()> {
    add(&mut delta.available_sub, amount)?;
    add(&mut delta.reserved_add, amount)
}
fn release(delta: &mut Delta, amount: u64) -> Result<()> {
    add(&mut delta.reserved_sub, amount)?;
    add(&mut delta.available_add, amount)
}
fn transfer(
    deltas: &mut BTreeMap<Key, Delta>,
    sender: Key,
    receiver: Key,
    amount: u64,
) -> Result<()> {
    let source = deltas.entry(sender).or_default();
    add(&mut source.reserved_sub, amount)?;
    add(&mut source.sent_add, amount)?;
    let target = deltas.entry(receiver).or_default();
    add(&mut target.available_add, amount)?;
    add(&mut target.received_add, amount)
}
fn update(value: u64, subtract: u64, increase: u64) -> Result<u64> {
    value
        .checked_sub(subtract)
        .and_then(|value| value.checked_add(increase))
        .ok_or_else(|| crate::StoreError::Limit.into())
}
pub(super) fn effects(record: JournalRecord, state: &State) -> Result<Effects> {
    let mut deltas: BTreeMap<Key, Delta> = BTreeMap::new();
    match record {
        JournalRecord::Supplies(value) => {
            let delta = deltas.entry(value.key()).or_default();
            match value {
                Operation::Issue(value) => {
                    add(&mut delta.available_add, value.amount)?;
                    add(&mut delta.minted_add, value.amount)?;
                }
                Operation::Burn(value) => {
                    add(&mut delta.available_sub, value.amount)?;
                    add(&mut delta.burned_add, value.amount)?;
                }
                Operation::Reserve(value) => lock(delta, value.amount)?,
            }
        }
        JournalRecord::ReserveOffer(value) => {
            lock(
                deltas
                    .entry(key(value.terms.maker, value.terms.give))
                    .or_default(),
                value.terms.give.amount,
            )?;
            lock(
                deltas
                    .entry(key(value.terms.taker, value.terms.want))
                    .or_default(),
                value.terms.want.amount,
            )?;
        }
        JournalRecord::CancelOffer(value) => {
            let original = outcome::cancel_target(state, &value)?;
            release(
                deltas
                    .entry(key(original.terms.maker, original.terms.give))
                    .or_default(),
                original.terms.give.amount,
            )?;
            release(
                deltas
                    .entry(key(original.terms.taker, original.terms.want))
                    .or_default(),
                original.terms.want.amount,
            )?;
        }
        JournalRecord::AcceptOffer(value) => {
            let original = outcome::accept_target(state, &value)?;
            transfer(
                &mut deltas,
                key(original.terms.maker, original.terms.give),
                key(original.terms.taker, original.terms.give),
                original.terms.give.amount,
            )?;
            transfer(
                &mut deltas,
                key(original.terms.taker, original.terms.want),
                key(original.terms.maker, original.terms.want),
                original.terms.want.amount,
            )?;
        }
    }
    if deltas.len() > 4 {
        return Err(SuppliesStoreError::Corrupt);
    }
    let mut balances = state.balances.clone();
    let mut flows = state.flows.clone();
    let mut batch = Batch {
        stock: Vec::new(),
        flows: Vec::new(),
    };
    for (key, delta) in deltas {
        let old = balances.get(&key).copied().unwrap_or_default();
        let old_flow = flows.get(&key).copied().unwrap_or_default();
        let stock = Balances {
            available: update(old.available, delta.available_sub, delta.available_add)?,
            reserved: update(old.reserved, delta.reserved_sub, delta.reserved_add)?,
            minted: update(old.minted, 0, delta.minted_add)?,
            burned: update(old.burned, 0, delta.burned_add)?,
            ..old
        };
        let flow = Flow {
            received: update(old_flow.received, 0, delta.received_add)?,
            sent: update(old_flow.sent, 0, delta.sent_add)?,
        };
        conservation(stock, flow)?;
        balances.insert(key, stock);
        if flow.nonzero() {
            flows.insert(key, flow);
        } else {
            flows.remove(&key);
        }
        batch.stock.push((key, stock));
        batch.flows.push((key, flow));
    }
    if balances.len() > CAPACITY as usize || flows.len() > CAPACITY as usize {
        return Err(crate::StoreError::Backpressure.into());
    }
    validate(&balances, &flows)?;
    Ok(Effects::Flow(batch))
}
