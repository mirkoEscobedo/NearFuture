use nf_kernel::{
    supplies::SuppliesPolicy,
    trade::{
        AcceptancePolicy, CancelOffer, OfferTerms, TradePolicy, TradeRejection,
        acceptance_policy_bytes, trade_policy_bytes,
    },
};
/// Private closed selection; Legacy2 keeps its exact old policy/body and physical profile6.
#[derive(Clone)]
pub(crate) enum SelectedPolicy {
    Legacy2(TradePolicy),
    Accepting3(AcceptancePolicy),
}
impl SelectedPolicy {
    pub(crate) fn base(&self) -> &TradePolicy {
        match self {
            Self::Legacy2(value) => value,
            Self::Accepting3(value) => &value.trade,
        }
    }
    pub(crate) fn supplies(&self) -> &SuppliesPolicy {
        &self.base().supplies
    }
    pub(crate) fn validate(&self) -> Result<(), TradeRejection> {
        match self {
            Self::Legacy2(value) => value.validate(),
            Self::Accepting3(value) => value.validate(),
        }
    }
    pub(crate) fn digest(&self) -> Result<[u8; 32], TradeRejection> {
        match self {
            Self::Legacy2(value) => value.digest(),
            Self::Accepting3(value) => value.digest(),
        }
    }
    pub(crate) fn bytes(&self) -> Result<Vec<u8>, TradeRejection> {
        match self {
            Self::Legacy2(value) => trade_policy_bytes(value),
            Self::Accepting3(value) => acceptance_policy_bytes(value),
        }
    }
    pub(crate) fn permit_offer(&self, value: &OfferTerms) -> Result<(), TradeRejection> {
        match self {
            Self::Legacy2(policy) => policy.permit_offer(value),
            Self::Accepting3(policy) => policy.permit_offer(value),
        }
    }
    pub(crate) fn permit_cancel(&self, value: &CancelOffer) -> Result<(), TradeRejection> {
        match self {
            Self::Legacy2(policy) => policy.permit_cancel(value),
            Self::Accepting3(policy) => policy.permit_cancel(value),
        }
    }
    pub(crate) fn accepting(&self) -> Option<&AcceptancePolicy> {
        match self {
            Self::Legacy2(_) => None,
            Self::Accepting3(value) => Some(value),
        }
    }
}
