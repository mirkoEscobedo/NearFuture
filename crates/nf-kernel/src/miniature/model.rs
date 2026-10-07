use alloc::{collections::BTreeMap, vec::Vec};
use nf_contract::identity::*;
use nf_world::{Schedule, State, WorldAction};

pub const MAX_MINIATURE_BYTES: usize = 1_048_576;
pub const MAX_MINIATURE_COMPONENT_BYTES: usize = 131072;
pub const MAX_MINIATURE_OUTCOMES: usize = 4096;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MiniatureRejection {
    InvalidReference = 1,
    DuplicateIdentity = 2,
    Limit = 3,
    InvalidValue = 4,
    UnsupportedProvider = 5,
    Unauthorized = 6,
    StaleRevision = 7,
    Conflict = 8,
    Overflow = 9,
    InvalidProposal = 10,
    ProviderFailed = 11,
    StaleSession = 12,
    FencedAuthority = 13,
    UnknownJob = 14,
    DuplicateJob = 15,
    InvalidFrontier = 16,
    StateMismatch = 17,
    Resources = 18,
    Locked = 19,
    AdmissionChanged = 20,
    Cancelled = 21,
}
impl From<nf_world::WorldError> for MiniatureRejection {
    fn from(value: nf_world::WorldError) -> Self {
        use nf_world::WorldError::*;
        match value {
            Unsupported => Self::UnsupportedProvider,
            Limit => Self::Limit,
            InvalidReference => Self::InvalidReference,
            Duplicate => Self::DuplicateIdentity,
            InvalidValue | Malformed => Self::InvalidValue,
            Unauthorized => Self::Unauthorized,
            StaleRevision => Self::StaleRevision,
            Conflict => Self::Conflict,
            Resources => Self::Resources,
            Locked => Self::Locked,
            Overflow => Self::Overflow,
        }
    }
}
pub type MiniatureResult<T> = Result<T, MiniatureRejection>;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MiniatureMetadata {
    pub tick: WorldTick,
    pub event_sequence: EventSeq,
    pub aggregate: AggregateId,
    pub provider: ProviderId,
    pub provider_aggregate: AggregateId,
    pub provider_revision: AggregateRevision,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureOutcome {
    pub operation: OperationId,
    pub job: JobId,
    pub rejection: Option<MiniatureRejection>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureWorld {
    pub(crate) metadata: MiniatureMetadata,
    pub(crate) component: State,
    pub(crate) outcomes: Vec<MiniatureOutcome>,
}
impl MiniatureWorld {
    pub fn new(metadata: MiniatureMetadata, component: State) -> MiniatureResult<Self> {
        let world = Self {
            metadata,
            component,
            outcomes: Vec::new(),
        };
        world.validate()?;
        Ok(world)
    }
    pub fn metadata(&self) -> MiniatureMetadata {
        self.metadata
    }
    pub fn component(&self) -> &State {
        &self.component
    }
    pub fn outcomes(&self) -> &[MiniatureOutcome] {
        &self.outcomes
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureIntent {
    pub request: RequestId,
    pub operation: OperationId,
    pub job: JobId,
    pub actor: AccountId,
    pub device: DeviceId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub provider: ProviderId,
    pub expected: BTreeMap<AggregateId, AggregateRevision>,
    pub action: WorldAction,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MiniatureHold {
    pub(crate) operation: OperationId,
    pub(crate) faction: EntityId,
    pub(crate) credits: u64,
    pub(crate) supplies: u64,
}
impl MiniatureHold {
    pub fn operation(self) -> OperationId {
        self.operation
    }
    pub fn faction(self) -> EntityId {
        self.faction
    }
    pub fn credits(self) -> u64 {
        self.credits
    }
    pub fn supplies(self) -> u64 {
        self.supplies
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureFrontier {
    pub(crate) snapshot: MiniatureWorld,
    pub(crate) input_hash: [u8; 32],
    pub(crate) authority: crate::AuthorityContext,
    pub(crate) membership_revision: u64,
    pub(crate) committing_tick: WorldTick,
    pub(crate) intents: Vec<MiniatureIntent>,
    pub(crate) outcomes: Vec<MiniatureOutcome>,
    pub(crate) plan: Option<nf_world::ActionPlan>,
}
impl MiniatureFrontier {
    pub fn snapshot(&self) -> &MiniatureWorld {
        &self.snapshot
    }
    pub fn intents(&self) -> &[MiniatureIntent] {
        &self.intents
    }
    pub fn outcomes(&self) -> &[MiniatureOutcome] {
        &self.outcomes
    }
    pub fn authority(&self) -> crate::AuthorityContext {
        self.authority
    }
    pub fn membership_revision(&self) -> u64 {
        self.membership_revision
    }
    pub fn committing_tick(&self) -> WorldTick {
        self.committing_tick
    }
    pub fn input_hash(&self) -> [u8; 32] {
        self.input_hash
    }
    pub fn reservation(&self) -> Option<MiniatureHold> {
        self.plan.as_ref().and_then(|p| {
            let h = p.reservation();
            (h.credits() != 0 || h.supplies() != 0).then_some(MiniatureHold {
                operation: h.operation(),
                faction: h.faction(),
                credits: h.credits(),
                supplies: h.supplies(),
            })
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MiniatureDisposition {
    AdvanceTick,
    CancelPending,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureActionEvent {
    pub(crate) request: RequestId,
    pub(crate) operation: OperationId,
    pub(crate) job: JobId,
    pub(crate) actor: AccountId,
    pub(crate) action: WorldAction,
    pub(crate) hold: MiniatureHold,
    pub(crate) created_schedule: Option<Schedule>,
}
impl MiniatureActionEvent {
    pub fn request(&self) -> RequestId {
        self.request
    }
    pub fn operation(&self) -> OperationId {
        self.operation
    }
    pub fn job(&self) -> JobId {
        self.job
    }
    pub fn actor(&self) -> AccountId {
        self.actor
    }
    pub fn action(&self) -> &WorldAction {
        &self.action
    }
    pub fn cost(&self) -> MiniatureHold {
        self.hold
    }
    pub fn created_schedule(&self) -> Option<&Schedule> {
        self.created_schedule.as_ref()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniatureBatch {
    pub(crate) before_hash: [u8; 32],
    pub(crate) after_hash: [u8; 32],
    pub(crate) disposition: MiniatureDisposition,
    pub(crate) tick: WorldTick,
    pub(crate) sequence: EventSeq,
    pub(crate) authority: crate::AuthorityContext,
    pub(crate) membership_revision: u64,
    pub(crate) admitted: Vec<MiniatureIntent>,
    pub(crate) action_event: Option<MiniatureActionEvent>,
    pub(crate) due: Vec<Schedule>,
    pub(crate) outcomes: Vec<MiniatureOutcome>,
}
impl MiniatureBatch {
    pub fn before_hash(&self) -> [u8; 32] {
        self.before_hash
    }
    pub fn after_hash(&self) -> [u8; 32] {
        self.after_hash
    }
    pub fn disposition(&self) -> MiniatureDisposition {
        self.disposition
    }
    pub fn tick(&self) -> WorldTick {
        self.tick
    }
    pub fn event_sequence(&self) -> EventSeq {
        self.sequence
    }
    pub fn authority(&self) -> crate::AuthorityContext {
        self.authority
    }
    pub fn membership_revision(&self) -> u64 {
        self.membership_revision
    }
    pub fn intents(&self) -> &[MiniatureIntent] {
        &self.admitted
    }
    pub fn action_event(&self) -> Option<&MiniatureActionEvent> {
        self.action_event.as_ref()
    }
    pub fn due_events(&self) -> &[Schedule] {
        &self.due
    }
    pub fn outcomes(&self) -> &[MiniatureOutcome] {
        &self.outcomes
    }
}
