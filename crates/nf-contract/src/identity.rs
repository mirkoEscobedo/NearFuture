//! Opaque identities and distinct monotonic counters. No display-name parsing or ID generation.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidIdLength;

macro_rules! identity {
    ($($name:ident),+ $(,)?) => { $(
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);
        impl $name {
            pub const fn from_bytes(bytes: [u8; 16]) -> Self { Self(bytes) }
            pub const fn as_bytes(&self) -> &[u8; 16] { &self.0 }
            pub fn from_slice(bytes: &[u8]) -> Result<Self, InvalidIdLength> {
                bytes.try_into().map(Self).map_err(|_| InvalidIdLength)
            }
        }
    )+ };
}
identity!(
    UniverseId,
    HistoryId,
    AccountId,
    DeviceId,
    ProviderId,
    JobId,
    CampaignId,
    BranchId,
    EntityId,
    AggregateId,
    RequestId,
    OperationId
);

macro_rules! counter {
    ($($name:ident),+ $(,)?) => { $(
        #[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u64);
        impl $name {
            pub fn checked_next(self) -> Option<Self> { self.0.checked_add(1).map(Self) }
        }
    )+ };
}
counter!(
    AuthorityTerm,
    RuntimeSession,
    RulesetRevision,
    WorldTick,
    EventSeq,
    AggregateRevision
);
