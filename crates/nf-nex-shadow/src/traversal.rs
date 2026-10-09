//! Copied-fact untargeted peace traversal; no game, manager, provider or effect calls.
//! Adapted DiplomacyBrain.checkPeace from Nexerelin a669f4d0740e95a4acbb6b894dde09ade67aa754.
//! Copyright (c) 2015 L.J. "Histidine" Lim; complete MIT notice: LICENSE-NEX-MIT.txt.
//! Source SHA-256 a8ec5be42031efaf2340bad199bdebbb435ed774ebfd7718e0b7ecbb50c04dcb.
use crate::Unavailable;
use alloc::{string::String, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportedPeaceReturn {
    Null,
    NonNull,
    NotSupplied,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalStep {
    RecentWar,
    CannotCeasefire,
    CommissionedPlayer,
    OffensiveBlocked,
    NullReturn,
    NonNullReturn,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversalEnemy {
    pub id: String,
    pub raw_weariness_bits: u32,
    pub recent_war: bool,
    pub can_ceasefire: bool,
    pub commissioned_player: bool,
    pub offensive_blocked: bool,
    pub reported_return: ReportedPeaceReturn,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversalVisit {
    pub enemy: String,
    pub step: TraversalStep,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeaceTraversalResult {
    pub ordered_enemies: Vec<String>,
    pub visits: Vec<TraversalVisit>,
    pub null_attempts: u8,
    pub returned_enemy: Option<String>,
    /// A singleton +0f requests the source refresh; this primitive performs no refresh.
    pub cache_refresh_bits: Vec<u32>,
}

/// Copied-fact untargeted traversal after the outer guards; no manager or effect calls.
/// All ranking bits are valid; copied IDs are nonempty and at most 128 UTF16 units.
pub fn evaluate_passed_outer_gates(
    supplied: &[TraversalEnemy],
) -> Result<PeaceTraversalResult, Unavailable> {
    if supplied.len() > 256 {
        return Err(Unavailable::Limit);
    }
    for enemy in supplied {
        if enemy.id.is_empty() {
            return Err(Unavailable::MissingFact);
        }
        if enemy.id.encode_utf16().take(129).count() > 128 {
            return Err(Unavailable::Limit);
        }
    }
    let mut ordered: Vec<_> = supplied.iter().collect();
    ordered.sort_by_key(|enemy| java_float_order_key(enemy.raw_weariness_bits));
    let mut result = PeaceTraversalResult {
        ordered_enemies: ordered.iter().map(|enemy| enemy.id.clone()).collect(),
        visits: Vec::new(),
        null_attempts: 0,
        returned_enemy: None,
        cache_refresh_bits: Vec::new(),
    };
    for enemy in ordered {
        let step = if enemy.recent_war {
            TraversalStep::RecentWar
        } else if !enemy.can_ceasefire {
            TraversalStep::CannotCeasefire
        } else if enemy.commissioned_player {
            TraversalStep::CommissionedPlayer
        } else if enemy.offensive_blocked {
            TraversalStep::OffensiveBlocked
        } else {
            match enemy.reported_return {
                ReportedPeaceReturn::NotSupplied => return Err(Unavailable::MissingFact),
                ReportedPeaceReturn::Null => TraversalStep::NullReturn,
                ReportedPeaceReturn::NonNull => TraversalStep::NonNullReturn,
            }
        };
        result.visits.push(TraversalVisit {
            enemy: enemy.id.clone(),
            step,
        });
        match step {
            TraversalStep::NonNullReturn => {
                result.returned_enemy = Some(enemy.id.clone());
                result.cache_refresh_bits.push(0x0000_0000);
                break;
            }
            TraversalStep::NullReturn => {
                result.null_attempts += 1;
                if result.null_attempts >= 3 {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(result)
}

// Match Java Float.compare: NaNs are equal after all numeric values; -0f precedes +0f.
// Only the ephemeral sort key is canonicalized; supplied raw bits remain untouched.
fn java_float_order_key(raw: u32) -> u32 {
    let bits = if (raw & 0x7fff_ffff) > 0x7f80_0000 {
        0x7fc0_0000
    } else {
        raw
    };
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits | 0x8000_0000
    }
}

/// Copied target selection after outer guards; supplied targets need not belong to the candidate pool.
/// Explicit targets use one supplied fact; absent targets delegate to the unchanged untargeted API.
pub fn evaluate_selection_passed_outer_gates(
    supplied: &[TraversalEnemy],
    supplied_target: Option<&TraversalEnemy>,
) -> Result<PeaceTraversalResult, Unavailable> {
    match supplied_target {
        Some(target) => evaluate_passed_outer_gates(core::slice::from_ref(target)),
        None => evaluate_passed_outer_gates(supplied),
    }
}
