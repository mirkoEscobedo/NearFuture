use crate::supplies::{Origin, RequestOutcome, SuppliesPolicy};
use alloc::vec::Vec;
use nf_contract::identity::*;

pub const MAX_ALLOWED_ORIGINS: usize = 64;
pub const MAX_TRADE_BODY: usize = 512;
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OfferId([u8; 16]);
impl OfferId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}
/// Explicit trusted-host scope; does not attest campaign detectors or native imports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeAdmission {
    AuthorityVaultOnly,
}
/// Trusted policy choice; neither the issue nor the design selects a cancelling party.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelRule {
    EitherNamedParty,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradePolicy {
    pub supplies: SuppliesPolicy,
    pub clock_authority: AccountId,
    pub allowed_origins: Vec<Origin>,
    pub admission: TradeAdmission,
    pub cancel_rule: CancelRule,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetTerms {
    pub content: [u8; 32],
    pub origin: Origin,
    pub amount: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfferTerms {
    pub offer: OfferId,
    pub version: u32,
    pub maker: AccountId,
    pub taker: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub policy: [u8; 32],
    pub give: AssetTerms,
    pub want: AssetTerms,
    pub expires_at: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReserveOffer {
    pub request: RequestId,
    pub operation: OperationId,
    pub maker_device: DeviceId,
    pub taker_device: DeviceId,
    pub terms: OfferTerms,
}
/// Names the original version and digest; carries no replacement terms or asset quantities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CancelOffer {
    pub request: RequestId,
    pub operation: OperationId,
    pub actor: AccountId,
    pub device: DeviceId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub policy: [u8; 32],
    pub offer: OfferId,
    pub version: u32,
    pub digest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeRejection {
    Scope,
    Policy,
    Unauthorized,
    UnsupportedContent,
    Limit,
    Conflict,
    Malformed,
    InsufficientAvailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeRequestOutcome {
    Supplies(RequestOutcome),
    Cancelled(TradeReceipt),
    Accepted(TradeReceipt),
    Reserved {
        operation: OperationId,
        offer: ReservedOffer,
    },
    Rejected {
        operation: OperationId,
        revision: u64,
        reason: TradeRejection,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReservedOffer {
    pub offer: OfferId,
    pub version: u32,
    pub revision: u64,
    pub digest: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfferStatusQuery {
    pub actor: AccountId,
    pub device: DeviceId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub offer: OfferId,
    pub version: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TradeOutboxQuery {
    pub actor: AccountId,
    pub device: DeviceId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub after_revision: u64,
    pub limit: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeFinalKind {
    Cancelled,
    Accepted,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OfferState {
    Reserved(ReservedOffer),
    Closed(TradeReceipt),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TradeReceipt {
    pub kind: TradeFinalKind,
    pub operation: OperationId,
    pub revision: u64,
    pub offer: OfferId,
    pub version: u32,
    pub digest: [u8; 32],
}
