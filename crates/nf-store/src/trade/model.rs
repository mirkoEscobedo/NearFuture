use crate::supplies::{ChallengeRequest, KnownSuppliesFrontiers};
use nf_identity::model::Scope;
use nf_kernel::{
    supplies::{BalanceQuery, Burn, Issuance, StatusQuery},
    trade::{AcceptOffer, CancelOffer, OfferStatusQuery, ReserveOffer, TradeOutboxQuery},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownTradeFrontiers {
    pub scope: Scope,
    pub revision: u64,
    pub membership_revision: u64,
    pub clock_tick: u64,
    pub clock_revision: u64,
}
impl KnownTradeFrontiers {
    pub(super) fn supplies(self) -> KnownSuppliesFrontiers {
        KnownSuppliesFrontiers {
            scope: self.scope,
            revision: self.revision,
            membership_revision: self.membership_revision,
        }
    }
}
#[derive(Clone, Copy)]
pub enum TradeChallenge<'a> {
    Issue(&'a Issuance),
    Burn(&'a Burn),
    Balance(&'a BalanceQuery),
    Status(&'a StatusQuery),
    ReserveMaker(&'a ReserveOffer),
    ReserveTaker(&'a ReserveOffer),
    Accept(&'a AcceptOffer),
    Cancel(&'a CancelOffer),
    OfferStatus(&'a OfferStatusQuery),
    Outbox(&'a TradeOutboxQuery),
}
impl<'a> TradeChallenge<'a> {
    pub(super) fn supplies(self) -> Option<ChallengeRequest<'a>> {
        match self {
            Self::Issue(value) => Some(ChallengeRequest::Issue(value)),
            Self::Burn(value) => Some(ChallengeRequest::Burn(value)),
            Self::Balance(value) => Some(ChallengeRequest::Balance(value)),
            Self::Status(value) => Some(ChallengeRequest::Status(value)),
            _ => None,
        }
    }
}
