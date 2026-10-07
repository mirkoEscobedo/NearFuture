use alloc::{string::String, vec::Vec};
use nf_nex_boundary::{DoubleBits, FloatBits};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuppliedDraw {
    pub purpose: String,
    pub value: DoubleBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeaceGates {
    pub has_enemies: bool,
    pub pirate: bool,
    pub allow_pirate: bool,
    pub days: FloatBits,
    pub minimum_interval: FloatBits,
    pub recent_war: bool,
    pub can_ceasefire: bool,
    pub commissioned: bool,
    pub offensive_blocked: bool,
    pub offensive_facts_provided: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeaceRules {
    pub minimum: FloatBits,
    pub divisor: FloatBits,
    pub divisor_per_level: FloatBits,
    pub player_level: u32,
    pub event_multiplier: FloatBits,
    pub treaty_chance: FloatBits,
    pub ceasefire_reduction: FloatBits,
    pub treaty_reduction: FloatBits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeaceFacts {
    pub faction: String,
    pub enemy: String,
    pub enemy_is_player: bool,
    pub treaty_relation: bool,
    pub own_weariness: FloatBits,
    pub enemy_weariness: FloatBits,
    pub own_events: FloatBits,
    pub enemy_events: FloatBits,
    pub gates: PeaceGates,
    pub rules: PeaceRules,
    pub draws: Vec<SuppliedDraw>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeaceDecision {
    None,
    Ceasefire,
    Treaty,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectCall {
    DiplomacyEvent {
        faction: String,
        enemy: String,
        event_id: String,
    },
    Weariness {
        faction: String,
        amount: FloatBits,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeaceResult {
    pub decision: PeaceDecision,
    pub consumed_draws: u8,
    pub effects: Vec<EffectCall>,
}
