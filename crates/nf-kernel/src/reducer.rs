use crate::*;

pub(crate) fn reduce_events(world: &World, events: &[Event]) -> Result<World, Rejection> {
    let mut next = world.clone();
    for event in events {
        let aggregate = match event {
            Event::RelationChanged { relation, delta } => {
                let target = next
                    .spec
                    .relations
                    .iter_mut()
                    .find(|v| v.id == *relation)
                    .ok_or(Rejection::InvalidReference)?;
                let score = target
                    .score
                    .checked_add(*delta)
                    .ok_or(Rejection::Overflow)?;
                if !(-10000..=10000).contains(&score) {
                    return Err(Rejection::Overflow);
                }
                target.score = score;
                target.aggregate
            }
            Event::MarketAdjusted { market, delta } => {
                let target = next
                    .spec
                    .markets
                    .iter_mut()
                    .find(|v| v.id == *market)
                    .ok_or(Rejection::InvalidReference)?;
                target.credits = target
                    .credits
                    .checked_add_signed(*delta)
                    .ok_or(Rejection::Overflow)?;
                target.aggregate
            }
            Event::ProviderAdvanced {
                provider,
                draws,
                cooldown_until,
            } => {
                let target = next
                    .spec
                    .providers
                    .iter_mut()
                    .find(|v| v.id == *provider)
                    .ok_or(Rejection::InvalidReference)?;
                if *draws < target.draws || *cooldown_until < target.cooldown_until {
                    return Err(Rejection::InvalidValue);
                }
                target.draws = *draws;
                target.cooldown_until = *cooldown_until;
                target.aggregate
            }
        };
        let revision = next
            .spec
            .revisions
            .get_mut(&aggregate)
            .ok_or(Rejection::InvalidReference)?;
        *revision = revision.checked_next().ok_or(Rejection::Overflow)?;
    }
    Ok(next)
}
pub(crate) fn finish_tick(mut world: World, outcomes: &[Outcome]) -> Result<World, Rejection> {
    if world.outcomes.len() + outcomes.len() > MAX_OUTCOMES {
        return Err(Rejection::Limit);
    }
    world.spec.tick = world.spec.tick.checked_next().ok_or(Rejection::Overflow)?;
    world.spec.event_seq = world
        .spec
        .event_seq
        .checked_next()
        .ok_or(Rejection::Overflow)?;
    world.outcomes.extend_from_slice(outcomes);
    Ok(world)
}
