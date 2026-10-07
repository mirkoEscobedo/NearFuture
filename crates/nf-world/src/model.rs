use alloc::vec::Vec;
use nf_contract::identity::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldError {
    Unsupported,
    Limit,
    InvalidReference,
    Duplicate,
    InvalidValue,
    Unauthorized,
    StaleRevision,
    Conflict,
    Resources,
    Locked,
    Overflow,
    Malformed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Genesis {
    pub universe: UniverseId,
    pub history: HistoryId,
    pub seed: [u8; 32],
    pub accounts: [AccountId; 3],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct System {
    pub id: EntityId,
    pub ordinal: u8,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Faction {
    pub id: EntityId,
    pub ordinal: u8,
    pub account: AccountId,
    pub credits: u64,
    pub supplies: u64,
    pub spent_credits: u64,
    pub spent_supplies: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    pub id: EntityId,
    pub ordinal: u8,
    pub system: EntityId,
    pub owner: Option<EntityId>,
    pub industries: [IndustryState; 2],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fleet {
    pub id: EntityId,
    pub faction: EntityId,
    pub location: FleetLocation,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub(crate) genesis: Genesis,
    pub(crate) revision: AggregateRevision,
    pub(crate) systems: Vec<System>,
    pub(crate) factions: Vec<Faction>,
    pub(crate) markets: Vec<Market>,
    pub(crate) fleets: Vec<Fleet>,
    pub(crate) relations: Vec<Relation>,
    pub(crate) schedules: Vec<Schedule>,
}
impl State {
    pub fn genesis(&self) -> Genesis {
        self.genesis
    }
    pub fn schedules(&self) -> &[Schedule] {
        &self.schedules
    }
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }
    pub fn revision(&self) -> AggregateRevision {
        self.revision
    }
    pub fn systems(&self) -> &[System] {
        &self.systems
    }
    pub fn factions(&self) -> &[Faction] {
        &self.factions
    }
    pub fn markets(&self) -> &[Market] {
        &self.markets
    }
    pub fn fleets(&self) -> &[Fleet] {
        &self.fleets
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndustryState {
    Absent,
    Constructing(EntityId),
    Complete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetLocation {
    Docked(EntityId),
    InTransit(EntityId),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relation {
    pub id: EntityId,
    pub left: EntityId,
    pub right: EntityId,
    pub score: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleKind {
    Industry {
        market: EntityId,
        industry: crate::IndustryKind,
    },
    Arrival {
        fleet: EntityId,
        origin: EntityId,
        destination: EntityId,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schedule {
    pub id: EntityId,
    pub operation: OperationId,
    pub due: WorldTick,
    pub kind: ScheduleKind,
}
