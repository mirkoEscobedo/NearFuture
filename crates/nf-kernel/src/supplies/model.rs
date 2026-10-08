use alloc::vec::Vec;
use nf_contract::identity::*;

pub const SUPPLIES_CONTENT: [u8; 32] = [0x53; 32];
pub const MAX_ISSUER_RULES: usize = 64;
/// Private ledger operation tags; these register no public wire capability.
pub const ISSUE_OPERATION: u32 = 0x5355_0001;
pub const BURN_OPERATION: u32 = 0x5355_0002;
pub const RESERVE_OPERATION: u32 = 0x5355_0003;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TrustClass {
    Sandbox,
    CooperativeAudited,
    Canonical,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssuanceReason {
    AuthorityGrant,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BurnReason {
    AuthorityDestruction,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Origin {
    pub trust: TrustClass,
    pub lineage: [u8; 32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IssuerRule {
    pub issuer: AccountId,
    pub content: [u8; 32],
    pub origin: Origin,
    pub reason: IssuanceReason,
    pub maximum: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BurnRule {
    pub issuer: AccountId,
    pub content: [u8; 32],
    pub origin: Origin,
    pub reason: BurnReason,
    pub maximum: u64,
}
/// Selected explicitly by the trusted host; never inferred from PLAYER/worker roles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuppliesPolicy {
    pub universe: UniverseId,
    pub history: HistoryId,
    pub ruleset: [u8; 32],
    pub issuers: Vec<IssuerRule>,
    pub burners: Vec<BurnRule>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuppliesRejection {
    Scope,
    Policy,
    Unauthorized,
    UnsupportedContent,
    Limit,
    Conflict,
    Malformed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Issuance {
    pub request: RequestId,
    pub issuance: OperationId,
    pub actor: AccountId,
    pub device: DeviceId,
    pub beneficiary: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub policy: [u8; 32],
    pub content: [u8; 32],
    pub origin: Origin,
    pub reason: IssuanceReason,
    pub amount: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalanceQuery {
    pub actor: AccountId,
    pub device: DeviceId,
    pub owner: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub content: [u8; 32],
    pub origin: Origin,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Balances {
    pub available: u64,
    pub reserved: u64,
    pub pending: u64,
    pub externalized: u64,
    pub minted: u64,
    pub burned: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IssuanceOutcome {
    pub issuance: OperationId,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Burn {
    pub request: RequestId,
    pub burn: OperationId,
    pub actor: AccountId,
    pub device: DeviceId,
    pub owner: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub policy: [u8; 32],
    pub content: [u8; 32],
    pub origin: Origin,
    pub reason: BurnReason,
    pub amount: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BurnOutcome {
    pub burn: OperationId,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusQuery {
    pub actor: AccountId,
    pub device: DeviceId,
    pub owner: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub request: RequestId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestOutcome {
    Issued(IssuanceOutcome),
    Burned(BurnOutcome),
    Reserved(ReserveOutcome),
    Rejected(RejectedOutcome),
}
/// A durable business decision, distinct from authorization and storage failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestRejection {
    InsufficientAvailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RejectedOutcome {
    pub operation: OperationId,
    pub revision: u64,
    pub reason: RequestRejection,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reserve {
    pub request: RequestId,
    pub reservation: OperationId,
    pub actor: AccountId,
    pub device: DeviceId,
    pub owner: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub policy: [u8; 32],
    pub content: [u8; 32],
    pub origin: Origin,
    pub amount: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReserveOutcome {
    pub reservation: OperationId,
    pub revision: u64,
}
