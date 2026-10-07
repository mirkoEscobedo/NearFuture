use crate::{ConcernClass, ExistingConcern, PriorityFacts, Unavailable, WarFacts, WarOperation};
use nf_contract::identity::EntityId;
use nf_nex_boundary::NexWorld;
/// Reads admitted owned values only. It does not certify capture or fabricate missing source facts.
pub fn war_from_world(
    world: &NexWorld,
    operation: WarOperation,
    target: Option<EntityId>,
) -> Result<WarFacts, Unavailable> {
    let s = world.snapshot();
    if !s.weariness.manager_present || !s.weariness.map_entry_present {
        return Err(Unavailable::MissingFact);
    }
    if !s.concern_config.enabled
        || operation == WarOperation::Generate && s.concern_config.no_auto_generate
    {
        return Err(Unavailable::Disabled);
    }
    let faction = s
        .factions
        .iter()
        .find(|f| f.id == s.subject_faction)
        .ok_or(Unavailable::MissingFact)?;
    let state = match target {
        Some(id) => Some(
            s.concerns
                .iter()
                .find(|c| c.instance == id)
                .ok_or(Unavailable::MissingFact)?,
        ),
        None if operation == WarOperation::Generate => None,
        None => return Err(Unavailable::MissingFact),
    };
    let facts = WarFacts {
        weariness: s.weariness.adjusted,
        minimum: s.weariness.minimum_for_peace,
        already_ended: state.is_some_and(|v| v.ended),
        current_action: state.is_some_and(|v| v.action.is_some()),
        existing: s
            .concerns
            .iter()
            .map(|v| ExistingConcern {
                class: ConcernClass::WarWeariness,
                ended: v.ended,
            })
            .collect(),
        priority: PriorityFacts {
            alignment: faction.diplomatic_alignment,
            max_alignment: s.priority_rules.max_alignment_modifier,
            positive_trait: s.priority_rules.positive_trait_multiplier,
            negative_trait: s.priority_rules.negative_trait_multiplier,
            traits: faction.traits.clone(),
            faction_multiplier: faction
                .priority_multipliers
                .iter()
                .find(|(id, _)| id == "warWeariness")
                .map(|(_, v)| *v),
            tags: s.concern_config.tags.clone(),
            existing: state.map_or_else(alloc::vec::Vec::new, |v| v.priority.clone()),
        },
    };
    crate::validation::war(&facts)?;
    Ok(facts)
}
/// Frozen boundary cannot supply every selected peace predicate and exact draw purpose.
pub fn selected_peace_from_world(_world: &NexWorld) -> Result<(), Unavailable> {
    Err(Unavailable::MissingFact)
}

/// Binds supplied provenance and named subject/instance to supported copied facts for inspection.
/// The digest omits unused snapshot fields and other concern instance IDs; it is not a full-world commitment.
/// Guarded acceptance remains unavailable without a reviewed complete world commitment.
pub fn evaluate_world_war(
    metadata: &crate::ShadowMetadata,
    world: &NexWorld,
    operation: WarOperation,
    target: Option<EntityId>,
) -> Result<crate::ShadowEvaluation, Unavailable> {
    if metadata.provenance != world.snapshot().provenance
        || metadata.subject_faction != world.snapshot().subject_faction
        || metadata.concern_instance != target
    {
        return Err(Unavailable::Stale);
    }
    let facts = war_from_world(world, operation, target)?;
    crate::evaluate_shadow(metadata, &crate::ShadowInput::War { operation, facts })
        .map(crate::ShadowEvaluation::require_world_commitment)
}
