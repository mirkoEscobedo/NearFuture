use crate::*;
use alloc::collections::BTreeSet;
/// Replay committed events only. Authentication and durable ordering belong to the composition driver.
pub fn apply_batch(world: &World, batch: &CommittedBatch) -> Result<World, Rejection> {
    if state_hash(world)? != batch.before_hash
        || world.spec.tick.checked_next() != Some(batch.tick)
        || world.spec.event_seq.checked_next() != Some(batch.sequence)
    {
        return Err(Rejection::StateMismatch);
    }
    if batch.admitted.len() != batch.outcomes.len() || batch.events.len() > MAX_FRONTIER * 2 {
        return Err(Rejection::InvalidFrontier);
    }
    let frontier = admit(world, batch.admitted.clone(), batch.authority)?;
    if frontier
        .jobs
        .iter()
        .map(|v| &v.intent)
        .ne(batch.admitted.iter())
        || frontier
            .jobs
            .iter()
            .zip(&batch.outcomes)
            .any(|(job, outcome)| {
                job.id() != outcome.job || job.intent.operation != outcome.operation
            })
    {
        return Err(Rejection::InvalidFrontier);
    }
    let mut next = world.clone();
    let mut event_index = 0;
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    for (job, outcome) in frontier.jobs.iter().zip(&batch.outcomes) {
        if outcome.rejection.is_some() {
            continue;
        }
        job.validation?;
        if job.reads.keys().any(|id| writes.contains(id))
            || job
                .writes
                .keys()
                .any(|id| reads.contains(id) || writes.contains(id))
        {
            return Err(Rejection::Conflict);
        }
        let pair = batch
            .events
            .get(event_index..event_index + 2)
            .ok_or(Rejection::InvalidProposal)?;
        validate_committed_pair(world, job, pair, batch)?;
        next = crate::reducer::reduce_events(&next, pair)?;
        reads.extend(job.reads.keys().copied());
        writes.extend(job.writes.keys().copied());
        event_index += 2;
    }
    if event_index != batch.events.len() {
        return Err(Rejection::InvalidProposal);
    }
    let next = crate::reducer::finish_tick(next, &batch.outcomes)?;
    if state_hash(&next)? != batch.after_hash {
        return Err(Rejection::StateMismatch);
    }
    Ok(next)
}
fn validate_committed_pair(
    world: &World,
    job: &Job,
    pair: &[Event],
    batch: &CommittedBatch,
) -> Result<(), Rejection> {
    let valid = match (&job.intent.command, &pair[0]) {
        (Command::SeekPeace { relation: a }, Event::RelationChanged { relation: b, delta }) => {
            a == b && (1..=10).contains(delta)
        }
        (
            Command::AdjustRelation {
                relation: a,
                delta: a_delta,
            },
            Event::RelationChanged {
                relation: b,
                delta: b_delta,
            },
        ) => a == b && a_delta == b_delta,
        (
            Command::AdjustMarket {
                market: a,
                delta: a_delta,
            },
            Event::MarketAdjusted {
                market: b,
                delta: b_delta,
            },
        ) => a == b && a_delta == b_delta,
        _ => false,
    };
    let module = world
        .view()
        .provider(job.intent.provider)
        .ok_or(Rejection::InvalidReference)?;
    let expected_draws = if matches!(job.intent.command, Command::SeekPeace { .. }) {
        module.draws.checked_add(1).ok_or(Rejection::Overflow)?
    } else {
        module.draws
    };
    if !valid
        || pair[1]
            != (Event::ProviderAdvanced {
                provider: job.intent.provider,
                draws: expected_draws,
                cooldown_until: batch.tick,
            })
        || module.cooldown_until > world.spec.tick
    {
        return Err(Rejection::InvalidProposal);
    }
    Ok(())
}
