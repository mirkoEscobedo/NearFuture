use super::{CancelOffer, OfferTerms, TradePolicy, TradeRejection, trade_policy_bytes};
use crate::supplies::Balances;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptRule {
    NamedTaker,
}
/// Additive policy selection; exact Legacy POLICY2 bytes remain embedded and unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptancePolicy {
    pub trade: TradePolicy,
    pub accept_rule: AcceptRule,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TradeBalance {
    pub stock: Balances,
    pub received: u64,
    pub sent: u64,
}
/// Fixed target-only204B body; cannot propose replacement assets or amounts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptOffer {
    pub request: nf_contract::identity::RequestId,
    pub operation: nf_contract::identity::OperationId,
    pub actor: nf_contract::identity::AccountId,
    pub device: nf_contract::identity::DeviceId,
    pub universe: nf_contract::identity::UniverseId,
    pub history: nf_contract::identity::HistoryId,
    pub policy: [u8; 32],
    pub offer: super::OfferId,
    pub version: u32,
    pub digest: [u8; 32],
}
impl AcceptancePolicy {
    pub fn validate(&self) -> Result<(), TradeRejection> {
        self.trade.validate()
    }
    pub fn digest(&self) -> Result<[u8; 32], TradeRejection> {
        Ok(Sha256::digest(acceptance_policy_bytes(self)?).into())
    }
    pub fn permit_offer(&self, value: &OfferTerms) -> Result<(), TradeRejection> {
        if value.policy != self.digest()? {
            return Err(TradeRejection::Policy);
        }
        let mut legacy = *value;
        legacy.policy = self.trade.digest()?;
        self.trade.permit_offer(&legacy)
    }
    pub fn permit_cancel(&self, value: &CancelOffer) -> Result<(), TradeRejection> {
        if value.policy != self.digest()? {
            return Err(TradeRejection::Policy);
        }
        let mut legacy = *value;
        legacy.policy = self.trade.digest()?;
        self.trade.permit_cancel(&legacy)
    }
    pub fn permit_accept(&self, value: &AcceptOffer) -> Result<(), TradeRejection> {
        self.validate()?;
        if value.universe != self.trade.supplies.universe
            || value.history != self.trade.supplies.history
        {
            return Err(TradeRejection::Scope);
        }
        if value.policy != self.digest()? {
            return Err(TradeRejection::Policy);
        }
        if value.version == 0 {
            return Err(TradeRejection::Limit);
        }
        match self.accept_rule {
            AcceptRule::NamedTaker => Ok(()),
        }
    }
}
pub fn acceptance_policy_bytes(policy: &AcceptancePolicy) -> Result<Vec<u8>, TradeRejection> {
    policy.validate()?;
    let legacy = trade_policy_bytes(&policy.trade)?;
    let mut bytes = Vec::from(b"NF-TRADE-POLICY-3\0".as_slice());
    bytes.extend_from_slice(&(legacy.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&legacy);
    let rule = match policy.accept_rule {
        AcceptRule::NamedTaker => 1_u32,
    };
    bytes.extend_from_slice(&rule.to_le_bytes());
    Ok(bytes)
}
