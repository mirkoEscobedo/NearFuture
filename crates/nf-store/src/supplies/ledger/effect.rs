use super::*;
use nf_kernel::trade::{CancelOffer, ReserveOffer};
type Effect = (Key, Balances);
/// A complete, checked balance update batch; no caller can observe half a joint reservation.
pub(in crate::supplies) enum Effects {
    None,
    One(Effect),
    Two([Effect; 2]),
    Flow(flow::Batch),
}
impl Effects {
    pub fn iter(&self) -> std::slice::Iter<'_, Effect> {
        match self {
            Self::None => [].iter(),
            Self::One(value) => std::slice::from_ref(value).iter(),
            Self::Two(values) => values.iter(),
            Self::Flow(batch) => batch.stock.iter(),
        }
    }
    pub fn flows(&self) -> &[(Key, flow::Flow)] {
        match self {
            Self::Flow(batch) => &batch.flows,
            _ => &[],
        }
    }
    pub fn supplies(value: Operation, balances: &BTreeMap<Key, Balances>) -> Result<Self> {
        let key = value.key();
        let mut balance = balances.get(&key).copied().unwrap_or_default();
        value.apply(&mut balance)?;
        Ok(Self::One((key, balance)))
    }
    pub fn cancel_offer(value: CancelOffer, state: &State) -> Result<Self> {
        let original = outcome::cancel_target(state, &value)?;
        let maker = (
            original.terms.maker,
            original.terms.give.content,
            original.terms.give.origin,
        );
        let taker = (
            original.terms.taker,
            original.terms.want.content,
            original.terms.want.origin,
        );
        if maker == taker {
            return Err(SuppliesStoreError::Corrupt);
        }
        let mut maker_balance = state
            .balances
            .get(&maker)
            .copied()
            .ok_or(SuppliesStoreError::Corrupt)?;
        let mut taker_balance = state
            .balances
            .get(&taker)
            .copied()
            .ok_or(SuppliesStoreError::Corrupt)?;
        release_units(&mut maker_balance, original.terms.give.amount)?;
        release_units(&mut taker_balance, original.terms.want.amount)?;
        Ok(Self::Two([(maker, maker_balance), (taker, taker_balance)]))
    }
    pub fn reserve_offer(value: ReserveOffer, balances: &BTreeMap<Key, Balances>) -> Result<Self> {
        let maker = (
            value.terms.maker,
            value.terms.give.content,
            value.terms.give.origin,
        );
        let taker = (
            value.terms.taker,
            value.terms.want.content,
            value.terms.want.origin,
        );
        if maker == taker {
            return Err(SuppliesStoreError::Corrupt);
        }
        let mut maker_balance = balances.get(&maker).copied().unwrap_or_default();
        let mut taker_balance = balances.get(&taker).copied().unwrap_or_default();
        reserve_units(&mut maker_balance, value.terms.give.amount)?;
        reserve_units(&mut taker_balance, value.terms.want.amount)?;
        Ok(Self::Two([(maker, maker_balance), (taker, taker_balance)]))
    }
}

/// Release the original reservation amount, preserving every other bucket and journal effect.
fn release_units(value: &mut Balances, amount: u64) -> Result<()> {
    let next = Balances {
        available: value
            .available
            .checked_add(amount)
            .ok_or(SuppliesStoreError::Corrupt)?,
        reserved: value
            .reserved
            .checked_sub(amount)
            .ok_or(SuppliesStoreError::Corrupt)?,
        ..*value
    };
    conservation(next)?;
    *value = next;
    Ok(())
}
