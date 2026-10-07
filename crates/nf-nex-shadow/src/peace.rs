//! Selected nonplayer DiplomacyBrain branch, pinned Nexerelin source; immutable requested-call data only.
use crate::{
    EffectCall, PeaceDecision, PeaceFacts, PeaceResult, Unavailable,
    validation::{f, finite},
};
use alloc::{string::ToString, vec, vec::Vec};
pub fn evaluate_selected_peace(input: &PeaceFacts) -> Result<PeaceResult, Unavailable> {
    validate(input)?;
    let g = &input.gates;
    let r = &input.rules;
    if input.enemy_is_player {
        return Err(Unavailable::Unsupported);
    }
    if !g.offensive_facts_provided {
        return Err(Unavailable::MissingFact);
    }
    if !g.has_enemies
        || g.pirate && !g.allow_pirate
        || f(g.days) < f(g.minimum_interval)
        || f(input.own_weariness) < f(r.minimum)
        || g.recent_war
        || !g.can_ceasefire
        || g.commissioned
        || g.offensive_blocked
        || f(input.enemy_weariness) < f(r.minimum)
    {
        return Ok(none(0));
    }
    let mut sum = f(input.own_weariness) + f(input.enemy_weariness);
    let mut events = f(input.own_events) + f(input.enemy_events);
    events *= f(r.event_multiplier);
    sum += events;
    let product = f(r.divisor_per_level) * (r.player_level as f32);
    let divisor = f(r.divisor) + product;
    finite(sum)?;
    finite(divisor)?;
    if divisor <= 0.0 {
        return Err(Unavailable::Unsupported);
    }
    let chance = sum / divisor;
    finite(chance)?;
    if draw(input, 0, "peace-chance")? > f64::from(chance) {
        return Ok(none(1));
    }
    let mut treaty = false;
    let mut consumed = 1;
    if input.treaty_relation {
        treaty = draw(input, 1, "peace-treaty")? < f64::from(f(r.treaty_chance));
        consumed = 2;
    }
    let event = if treaty { "peace_treaty" } else { "ceasefire" };
    let reduction = if treaty {
        r.treaty_reduction
    } else {
        r.ceasefire_reduction
    };
    let amount = finite(-f(reduction))?;
    Ok(PeaceResult {
        decision: if treaty {
            PeaceDecision::Treaty
        } else {
            PeaceDecision::Ceasefire
        },
        consumed_draws: consumed,
        effects: vec![
            EffectCall::DiplomacyEvent {
                faction: input.faction.clone(),
                enemy: input.enemy.clone(),
                event_id: event.to_string(),
            },
            EffectCall::Weariness {
                faction: input.faction.clone(),
                amount,
            },
            EffectCall::Weariness {
                faction: input.enemy.clone(),
                amount,
            },
        ],
    })
}
pub(crate) fn validate(input: &PeaceFacts) -> Result<(), Unavailable> {
    crate::validation::text(&input.faction)?;
    crate::validation::text(&input.enemy)?;
    if input.faction == input.enemy || input.faction.len() > 128 || input.enemy.len() > 128 {
        return Err(Unavailable::Unsupported);
    }
    if input.draws.len() > 2 {
        return Err(Unavailable::Limit);
    }
    let r = &input.rules;
    if r.player_level > 10000
        || r.event_multiplier.bits() != 40_f32.to_bits()
        || r.treaty_chance.bits() != 0.3_f32.to_bits()
        || input.gates.minimum_interval.bits() != 30_f32.to_bits()
    {
        return Err(Unavailable::Unsupported);
    }
    for d in &input.draws {
        crate::validation::text(&d.purpose)?;
        let value = f64::from_bits(d.value.bits());
        if !(0.0..1.0).contains(&value) {
            return Err(Unavailable::Unsupported);
        }
    }
    Ok(())
}
fn draw(input: &PeaceFacts, ordinal: usize, purpose: &str) -> Result<f64, Unavailable> {
    let d = input.draws.get(ordinal).ok_or(Unavailable::MissingFact)?;
    if d.purpose != purpose {
        return Err(Unavailable::MissingFact);
    }
    Ok(f64::from_bits(d.value.bits()))
}
fn none(consumed: u8) -> PeaceResult {
    PeaceResult {
        decision: PeaceDecision::None,
        consumed_draws: consumed,
        effects: Vec::new(),
    }
}
