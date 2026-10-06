//! Pure bounded strategic state. Snapshot fields remain inaccessible to mutation after validation.
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use nf_contract::identity::*;

pub const MAX_ENTITIES: usize = 128;
pub const MAX_PROVIDERS: usize = 32;
pub const MAX_FRONTIER: usize = 64;
pub const MAX_OUTCOMES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Domain {
    Faction,
    Relation,
    Market,
    Provider,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    SeekPeace,
    AdjustRelation,
    AdjustMarket,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderKind {
    SyntheticPeace,
    Manual,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Faction {
    pub id: EntityId,
    pub aggregate: AggregateId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relation {
    pub id: EntityId,
    pub aggregate: AggregateId,
    pub left: EntityId,
    pub right: EntityId,
    pub score: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    pub id: EntityId,
    pub aggregate: AggregateId,
    pub faction: EntityId,
    pub credits: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderState {
    pub id: ProviderId,
    pub aggregate: AggregateId,
    pub draws: u64,
    pub cooldown_until: WorldTick,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipRecord {
    pub aggregate: AggregateId,
    pub provider: ProviderId,
    pub generation: u64,
    pub activation_seq: EventSeq,
    pub ruleset_hash: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderManifest {
    pub id: ProviderId,
    pub implementation_hash: [u8; 32],
    pub version: u32,
    pub state_schema: u32,
    pub kind: ProviderKind,
    pub read_domains: BTreeSet<Domain>,
    pub write_domains: BTreeSet<Domain>,
    pub capabilities: BTreeSet<Capability>,
    pub principals: BTreeSet<AccountId>,
    pub max_events: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    InvalidReference,
    DuplicateIdentity,
    Limit,
    InvalidValue,
    UnsupportedProvider,
    Unauthorized,
    StaleRevision,
    Conflict,
    Overflow,
    InvalidProposal,
    ProviderFailed,
    StaleSession,
    FencedAuthority,
    UnknownJob,
    DuplicateJob,
    InvalidFrontier,
    StateMismatch,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldSpec {
    pub universe: UniverseId,
    pub history: HistoryId,
    pub seed: [u8; 32],
    pub ruleset_hash: [u8; 32],
    pub tick: WorldTick,
    pub event_seq: EventSeq,
    pub factions: Vec<Faction>,
    pub relations: Vec<Relation>,
    pub markets: Vec<Market>,
    pub providers: Vec<ProviderState>,
    pub registry: Vec<ProviderManifest>,
    pub ownership: Vec<OwnershipRecord>,
    pub revisions: BTreeMap<AggregateId, AggregateRevision>,
}
impl WorldSpec {
    pub fn empty(
        universe: UniverseId,
        history: HistoryId,
        seed: [u8; 32],
        ruleset_hash: [u8; 32],
    ) -> Self {
        Self {
            universe,
            history,
            seed,
            ruleset_hash,
            tick: WorldTick(0),
            event_seq: EventSeq(0),
            factions: Vec::new(),
            relations: Vec::new(),
            markets: Vec::new(),
            providers: Vec::new(),
            registry: Vec::new(),
            ownership: Vec::new(),
            revisions: BTreeMap::new(),
        }
    }
}
