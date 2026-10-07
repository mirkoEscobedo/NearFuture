use crate::{PriorityFacts, Unavailable, WarFacts};
use alloc::collections::BTreeSet;
use nf_nex_boundary::{FloatBits, Modifier};
pub(crate) fn text(value: &str) -> Result<(), Unavailable> {
    if value.is_empty() || value.len() > 256 {
        return Err(Unavailable::Limit);
    }
    if value.chars().any(char::is_control)
        || !nf_contract::canonical::text::is_canonical_text(value)
    {
        return Err(Unavailable::Unsupported);
    }
    Ok(())
}
pub(crate) fn finite(value: f32) -> Result<FloatBits, Unavailable> {
    FloatBits::new(value.to_bits()).map_err(|_| Unavailable::NonFinite)
}
pub(crate) fn f(value: FloatBits) -> f32 {
    f32::from_bits(value.bits())
}
pub(crate) fn modifiers(values: &[Modifier]) -> Result<(), Unavailable> {
    if values.len() > 64 {
        return Err(Unavailable::Limit);
    }
    let mut ids = BTreeSet::new();
    for value in values {
        if value.id.len() > 128 {
            return Err(Unavailable::Limit);
        }
        text(&value.id)?;
        if !ids.insert((&value.id, value.kind as u8)) {
            return Err(Unavailable::Unsupported);
        }
    }
    Ok(())
}
pub(crate) fn priority(p: &PriorityFacts) -> Result<(), Unavailable> {
    if p.tags.len() != 5 || p.traits.len() > 64 {
        return Err(Unavailable::Unsupported);
    }
    for tag in &p.tags {
        text(tag)?;
    }
    let tags: BTreeSet<_> = p.tags.iter().map(|s| s.as_str()).collect();
    if tags
        != [
            "diplomacy",
            "canMakePeace",
            "trait_pacifist",
            "trait_weak-willed",
            "!trait_stalwart",
        ]
        .into_iter()
        .collect()
    {
        return Err(Unavailable::Unsupported);
    }
    let mut traits = BTreeSet::new();
    for value in &p.traits {
        if !matches!(value.as_str(), "pacifist" | "weak-willed" | "stalwart")
            || !traits.insert(value)
        {
            return Err(Unavailable::Unsupported);
        }
    }
    modifiers(&p.existing)
}
pub(crate) fn war(input: &WarFacts) -> Result<(), Unavailable> {
    if input.existing.len() > 256 {
        return Err(Unavailable::Limit);
    }
    priority(&input.priority)
}
