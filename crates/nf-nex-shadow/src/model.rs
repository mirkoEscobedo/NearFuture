use alloc::{string::String, vec::Vec};
use nf_nex_boundary::{FloatBits, Modifier};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unavailable {
    MissingFact,
    Unsupported,
    NonFinite,
    Limit,
    Stale,
    Disabled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcernClass {
    WarWeariness,
    Other,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExistingConcern {
    pub class: ConcernClass,
    pub ended: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriorityFacts {
    pub alignment: FloatBits,
    pub max_alignment: FloatBits,
    pub positive_trait: FloatBits,
    pub negative_trait: FloatBits,
    pub traits: Vec<String>,
    pub faction_multiplier: Option<FloatBits>,
    pub tags: Vec<String>,
    pub existing: Vec<Modifier>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WarFacts {
    pub weariness: FloatBits,
    pub minimum: FloatBits,
    pub already_ended: bool,
    pub current_action: bool,
    pub existing: Vec<ExistingConcern>,
    pub priority: PriorityFacts,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WarOperation {
    Generate,
    Update,
    IsValid,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WarResult {
    pub generated: bool,
    pub ended: bool,
    pub abort_current_action: bool,
    pub valid: bool,
    pub existing_priority: Vec<Modifier>,
    pub writes: Vec<Modifier>,
}
