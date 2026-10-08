use super::*;
use nf_kernel::trade::{
    AcceptOffer, CancelOffer, ReserveOffer, accept_offer_bytes, accept_offer_digest,
    cancel_offer_bytes, cancel_offer_digest, decode_accept_offer, decode_cancel_offer,
    decode_reserve_offer, economic_accept_offer_digest, economic_cancel_offer_digest,
    economic_reserve_offer_digest, reserve_offer_bytes, reserve_offer_digest,
};

/// Closed profile6 records share the existing request, operation and revision namespace.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::supplies) enum JournalRecord {
    Supplies(Operation),
    ReserveOffer(ReserveOffer),
    CancelOffer(CancelOffer),
    AcceptOffer(AcceptOffer),
}
impl JournalRecord {
    pub fn decode(
        bytes: &[u8],
        trade: Option<&crate::trade::policy::SelectedPolicy>,
    ) -> Result<Self> {
        if bytes.starts_with(b"NF-TRADE-ACCEPT-OFFER-1\0") {
            if trade.and_then(|policy| policy.accepting()).is_none() {
                return Err(SuppliesStoreError::Corrupt);
            }
            decode_accept_offer(bytes)
                .map(Self::AcceptOffer)
                .map_err(|_| SuppliesStoreError::Corrupt)
        } else if bytes.starts_with(b"NF-TRADE-RESERVE-OFFER-1\0") {
            if trade.is_none() {
                return Err(SuppliesStoreError::Corrupt);
            }
            decode_reserve_offer(bytes)
                .map(Self::ReserveOffer)
                .map_err(|_| SuppliesStoreError::Corrupt)
        } else if bytes.starts_with(b"NF-TRADE-CANCEL-OFFER-1\0") {
            if trade.is_none() {
                return Err(SuppliesStoreError::Corrupt);
            }
            decode_cancel_offer(bytes)
                .map(Self::CancelOffer)
                .map_err(|_| SuppliesStoreError::Corrupt)
        } else {
            Operation::decode(bytes).map(Self::Supplies)
        }
    }
    pub fn permit(
        self,
        policy: &SuppliesPolicy,
        trade: Option<&crate::trade::policy::SelectedPolicy>,
    ) -> Result<()> {
        match self {
            Self::Supplies(value) => value.permit(policy),
            Self::ReserveOffer(value) => trade
                .ok_or(SuppliesStoreError::Corrupt)?
                .permit_offer(&value.terms)
                .map_err(|_| SuppliesStoreError::Corrupt),
            Self::AcceptOffer(value) => trade
                .and_then(|policy| policy.accepting())
                .ok_or(SuppliesStoreError::Corrupt)?
                .permit_accept(&value)
                .map_err(|_| SuppliesStoreError::Corrupt),
            Self::CancelOffer(value) => trade
                .ok_or(SuppliesStoreError::Corrupt)?
                .permit_cancel(&value)
                .map_err(|_| SuppliesStoreError::Corrupt),
        }
    }
    pub fn id(self) -> OperationId {
        match self {
            Self::Supplies(v) => v.id(),
            Self::ReserveOffer(v) => v.operation,
            Self::CancelOffer(v) => v.operation,
            Self::AcceptOffer(v) => v.operation,
        }
    }
    pub fn request(self) -> RequestId {
        match self {
            Self::Supplies(v) => v.request(),
            Self::ReserveOffer(v) => v.request,
            Self::CancelOffer(v) => v.request,
            Self::AcceptOffer(v) => v.request,
        }
    }
    pub fn binding(self) -> [u8; 32] {
        match self {
            Self::Supplies(v) => v.binding(),
            Self::ReserveOffer(v) => reserve_offer_digest(&v),
            Self::CancelOffer(v) => cancel_offer_digest(&v),
            Self::AcceptOffer(v) => accept_offer_digest(&v),
        }
    }
    pub fn same_kind(self, other: Self) -> bool {
        match (self, other) {
            (Self::Supplies(a), Self::Supplies(b)) => {
                std::mem::discriminant(&a) == std::mem::discriminant(&b)
            }
            (Self::ReserveOffer(_), Self::ReserveOffer(_)) => true,
            (Self::CancelOffer(_), Self::CancelOffer(_)) => true,
            (Self::AcceptOffer(_), Self::AcceptOffer(_)) => true,
            _ => false,
        }
    }
    pub fn economic(self) -> [u8; 32] {
        match self {
            Self::Supplies(v) => v.economic(),
            Self::ReserveOffer(v) => economic_reserve_offer_digest(&v),
            Self::CancelOffer(v) => economic_cancel_offer_digest(&v),
            Self::AcceptOffer(v) => economic_accept_offer_digest(&v),
        }
    }
    pub fn bytes(self) -> Vec<u8> {
        match self {
            Self::Supplies(v) => v.bytes(),
            Self::ReserveOffer(v) => reserve_offer_bytes(&v),
            Self::CancelOffer(v) => cancel_offer_bytes(&v),
            Self::AcceptOffer(v) => accept_offer_bytes(&v),
        }
    }
    fn insufficient(self, balances: &BTreeMap<Key, Balances>) -> bool {
        match self {
            Self::Supplies(value) => {
                let available = balances
                    .get(&value.key())
                    .copied()
                    .unwrap_or_default()
                    .available;
                matches!(value, Operation::Reserve(v) if v.amount > available)
                    || matches!(value, Operation::Burn(v) if v.amount > available)
            }
            Self::ReserveOffer(value) => {
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
                value.terms.give.amount
                    > balances.get(&maker).copied().unwrap_or_default().available
                    || value.terms.want.amount
                        > balances.get(&taker).copied().unwrap_or_default().available
            }
            Self::CancelOffer(_) | Self::AcceptOffer(_) => false,
        }
    }

    pub fn decision(self, balances: &BTreeMap<Key, Balances>) -> Decision {
        if self.insufficient(balances) {
            Decision::InsufficientAvailable
        } else {
            Decision::Accepted
        }
    }
    /// Full pure effects validate the retained decision against preceding stock.
    pub fn effect(self, decision: Decision, state: &State) -> Result<Effects> {
        // Lifetime reuse is refused even if the proposed stock would otherwise be insufficient.
        if let Self::ReserveOffer(value) = self
            && state.offers.contains_key(&value.terms.offer)
        {
            return Err(SuppliesRejection::Conflict.into());
        }
        if decision == Decision::InsufficientAvailable {
            return if self.insufficient(&state.balances) {
                Ok(Effects::None)
            } else {
                Err(SuppliesStoreError::Corrupt)
            };
        }
        if state.mode == LedgerMode::Accepting {
            return flow::effects(self, state);
        }
        match self {
            Self::AcceptOffer(_) => Err(SuppliesStoreError::Corrupt),
            Self::Supplies(value) => Effects::supplies(value, &state.balances),
            Self::ReserveOffer(value) => Effects::reserve_offer(value, &state.balances),
            Self::CancelOffer(value) => Effects::cancel_offer(value, state),
        }
    }
}
