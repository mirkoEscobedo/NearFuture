use super::*;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use nf_world::Candidate;
/// Pure deterministic admission; the imperative owner must authenticate every intent separately.
pub fn admit_miniature(
    world: &MiniatureWorld,
    mut intents: Vec<MiniatureIntent>,
    authority: crate::AuthorityContext,
    membership_revision: u64,
) -> MiniatureResult<MiniatureFrontier> {
    world.validate()?;
    world
        .metadata
        .event_sequence
        .checked_next()
        .ok_or(MiniatureRejection::Overflow)?;
    if intents.len() > 64 {
        return Err(MiniatureRejection::Limit);
    }
    if authority.term.0 == 0 {
        return Err(MiniatureRejection::FencedAuthority);
    }
    if authority.session.0 == 0 {
        return Err(MiniatureRejection::StaleSession);
    }
    let committing_tick = world
        .metadata
        .tick
        .checked_next()
        .ok_or(MiniatureRejection::Overflow)?;
    let g = world.component.genesis();
    let m = world.metadata;
    let mut requests = BTreeSet::new();
    let mut operations = BTreeSet::new();
    let mut jobs = BTreeSet::new();
    for intent in &intents {
        encode_miniature_intent(intent)?;
        if intent.universe != g.universe || intent.history != g.history {
            return Err(MiniatureRejection::InvalidReference);
        }
        if intent.provider != m.provider {
            return Err(MiniatureRejection::UnsupportedProvider);
        }
        if intent.expected.keys().copied().collect::<BTreeSet<_>>()
            != BTreeSet::from([m.aggregate, m.provider_aggregate])
        {
            return Err(MiniatureRejection::InvalidReference);
        }
        if !requests.insert(intent.request)
            || !operations.insert(intent.operation)
            || !jobs.insert(intent.job)
            || world
                .outcomes
                .iter()
                .any(|v| v.operation == intent.operation || v.job == intent.job)
        {
            return Err(MiniatureRejection::DuplicateIdentity);
        }
    }
    if world
        .outcomes
        .len()
        .checked_add(intents.len())
        .is_none_or(|n| n > MAX_MINIATURE_OUTCOMES)
    {
        return Err(MiniatureRejection::Limit);
    }
    intents.sort_by_key(|i| {
        (
            i.provider,
            nf_world::action_target(&world.component, &i.action),
            i.operation,
            i.job,
        )
    });
    let mut candidates = Vec::new();
    let mut rejected = BTreeMap::new();
    for intent in &intents {
        if intent.expected[&m.provider_aggregate] != m.provider_revision {
            rejected.insert(intent.job, MiniatureRejection::StaleRevision);
            continue;
        }
        candidates.push(Candidate {
            request: intent.request,
            operation: intent.operation,
            job: intent.job,
            actor: intent.actor,
            expected_revision: intent.expected[&m.aggregate],
            action: intent.action.clone(),
        });
    }
    let plan = nf_world::plan_reservations(&world.component, committing_tick, &candidates)?;
    let planned: BTreeMap<_, _> = plan
        .outcomes()
        .iter()
        .map(|o| (o.job, o.rejection.map(Into::into)))
        .collect();
    let outcomes = intents
        .iter()
        .map(|intent| MiniatureOutcome {
            operation: intent.operation,
            job: intent.job,
            rejection: rejected
                .get(&intent.job)
                .copied()
                .or_else(|| planned[&intent.job]),
        })
        .collect();
    Ok(MiniatureFrontier {
        snapshot: world.clone(),
        input_hash: miniature_state_hash(world)?,
        authority,
        membership_revision,
        committing_tick,
        intents,
        outcomes,
        plan: plan.winner().cloned(),
    })
}
