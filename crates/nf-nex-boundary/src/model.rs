use crate::{Definition, DoubleBits, FloatBits};
use alloc::{string::String, vec::Vec};
use nf_contract::identity::{EntityId, EventSeq, HistoryId, RuntimeSession, UniverseId};

pub const SOURCE_COMMIT: &str = "a669f4d0740e95a4acbb6b894dde09ade67aa754";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observation {
    Synthetic,
    CapturedUnverified,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    pub source_commit: String,
    pub source_digest: [u8; 32],
    pub runtime_digest: [u8; 32],
    pub ruleset_digest: [u8; 32],
    pub merged_config_digest: [u8; 32],
    pub universe: UniverseId,
    pub history: HistoryId,
    pub runtime_session: RuntimeSession,
    pub frontier: EventSeq,
    pub observation: Observation,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Modifier {
    pub id: String,
    pub kind: ModifierKind,
    pub value: FloatBits,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModifierKind {
    Flat,
    Percent,
    Multiplier,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactionView {
    pub id: String,
    pub live: bool,
    pub traits: Vec<String>,
    pub diplomatic_alignment: FloatBits,
    pub priority_multipliers: Vec<(String, FloatBits)>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationView {
    pub from: String,
    pub to: String,
    pub relationship: FloatBits,
    pub disposition: Option<FloatBits>,
    pub hostile: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketStrength {
    pub id: String,
    pub size: u8,
    pub strength: FloatBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrengthView {
    pub faction: String,
    pub markets: Vec<MarketStrength>,
    pub fleet_strength: FloatBits,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemyFilter {
    AdjustedGetter { allow_pirates: bool },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WearinessView {
    pub manager_present: bool,
    pub map_entry_present: bool,
    pub raw: FloatBits,
    pub adjusted: FloatBits,
    pub enemies: Vec<String>,
    pub filter: EnemyFilter,
    pub minimum_for_peace: FloatBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcernConfig {
    pub definition: Definition,
    pub enabled: bool,
    pub no_auto_generate: bool,
    pub tags: Vec<String>,
    pub cooldown_multiplier: FloatBits,
    pub anti_repetition_multiplier: FloatBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionConfig {
    pub definition: Definition,
    pub enabled: bool,
    pub shim: bool,
    pub tags: Vec<String>,
    pub chance: FloatBits,
    pub cooldown: FloatBits,
    pub anti_repetition: FloatBits,
    pub repetition_id: Option<String>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionStatus {
    Starting,
    InProgress,
    Success,
    Failure,
    Cancelled,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionView {
    pub instance: EntityId,
    pub definition: Definition,
    pub status: ActionStatus,
    pub ended: bool,
    pub meetings_since_ended: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcernView {
    pub instance: EntityId,
    pub definition: Definition,
    pub ended: bool,
    pub cooldown: FloatBits,
    pub action: Option<ActionView>,
    pub target_faction: Option<String>,
    pub market_id: Option<String>,
    pub priority_base: FloatBits,
    pub priority: Vec<Modifier>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntervalView {
    pub id: String,
    pub elapsed: FloatBits,
    pub minimum: FloatBits,
    pub maximum: FloatBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomDraw {
    pub purpose: String,
    pub ordinal: u32,
    pub value: DoubleBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimerView {
    pub meeting: u64,
    pub advance_days: FloatBits,
    pub intervals: Vec<IntervalView>,
    pub draws: Vec<RandomDraw>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Registration {
    pub class_path: String,
    pub origin: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionInventory {
    pub complete: bool,
    pub listeners: Vec<Registration>,
    pub direct_calls: Vec<Registration>,
    /// Definition IDs outside the selected projection; always enumerated as legacy-only.
    pub other_definitions: Vec<Definition>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorityRules {
    pub max_alignment_modifier: FloatBits,
    pub positive_trait_multiplier: FloatBits,
    pub negative_trait_multiplier: FloatBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub provenance: Provenance,
    pub subject_faction: String,
    pub factions: Vec<FactionView>,
    pub relations: Vec<RelationView>,
    pub strengths: Vec<StrengthView>,
    pub weariness: WearinessView,
    pub priority_rules: PriorityRules,
    pub concern_config: ConcernConfig,
    pub action_configs: Vec<ActionConfig>,
    pub concerns: Vec<ConcernView>,
    pub timers: TimerView,
    pub extensions: ExtensionInventory,
}
