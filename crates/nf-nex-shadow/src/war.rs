//! Source-faithful selected WarWearinessConcern methods; returns ordered data, never effects.
//! Nexerelin a669f4d0740e95a4acbb6b894dde09ade67aa754, MIT copyright2015 L.J. "Histidine" Lim.
use crate::{
    ConcernClass, Unavailable, WarFacts, WarOperation, WarResult,
    validation::{f, finite},
};
use alloc::{string::ToString, vec::Vec};
use nf_nex_boundary::{Modifier, ModifierKind};
pub fn evaluate_war(input: &WarFacts, operation: WarOperation) -> Result<WarResult, Unavailable> {
    crate::validation::war(input)?;
    let threshold = f(input.minimum) * 0.75_f32;
    let valid = f(input.weariness) >= threshold;
    let mut result = WarResult {
        generated: false,
        ended: input.already_ended,
        abort_current_action: false,
        valid,
        existing_priority: input.priority.existing.clone(),
        writes: Vec::new(),
    };
    if operation == WarOperation::IsValid {
        return Ok(result);
    }
    if operation == WarOperation::Generate
        && input
            .existing
            .iter()
            .any(|v| v.class == ConcernClass::WarWeariness && !v.ended)
    {
        return Ok(result);
    }
    if f(input.weariness) < threshold {
        result.ended = true;
        result.abort_current_action = input.current_action;
    } else {
        result.writes = priority_writes(input)?;
    }
    if operation == WarOperation::Generate {
        result.generated = !result.ended;
    }
    Ok(result)
}
fn priority_writes(input: &WarFacts) -> Result<Vec<Modifier>, Unavailable> {
    let p = &input.priority;
    let mut writes = Vec::with_capacity(6);
    writes.push(entry(
        "value",
        ModifierKind::Flat,
        f(input.weariness) / 100_f32,
    )?);
    if p.tags.iter().any(|tag| tag == "diplomacy") {
        let product = f(p.max_alignment) * f(p.alignment);
        writes.push(entry(
            "alignment_diplomatic",
            ModifierKind::Multiplier,
            1_f32 + product,
        )?);
    }
    for tag in &p.tags {
        let (trait_id, multiplier) = if let Some(id) = tag.strip_prefix("trait_") {
            (id, p.positive_trait)
        } else if let Some(id) = tag.strip_prefix("!trait_") {
            (id, p.negative_trait)
        } else {
            continue;
        };
        if p.traits.iter().any(|id| id == trait_id) {
            writes.push(entry(
                &alloc::format!("trait_{trait_id}"),
                ModifierKind::Multiplier,
                f(multiplier),
            )?);
        }
    }
    if let Some(value) = p.faction_multiplier {
        writes.push(entry("faction", ModifierKind::Multiplier, f(value))?);
    }
    Ok(writes)
}
fn entry(id: &str, kind: ModifierKind, value: f32) -> Result<Modifier, Unavailable> {
    Ok(Modifier {
        id: id.to_string(),
        kind,
        value: finite(value)?,
    })
}
