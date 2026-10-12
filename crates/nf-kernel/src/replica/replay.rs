//! Opt-in committed-batch data replay under one borrowed cumulative scope.
//! Authentication, durable ordering and candidate installation belong to the driver.
use crate::{CommittedBatch, Outcome, Rejection, World, MAX_FRONTIER, MAX_OUTCOMES};
use alloc::collections::BTreeSet;
use nf_contract::canonical::replica_budget::{
    BudgetResult, ReplicaDecodeScope, ScopedError, ScopedResult,
};
use nf_contract::identity::AggregateId;

/// Replays into a new candidate World; this grants no writer or install authority.
/// Logical charges cover actual owned replay copies and canonical hash payloads.
/// Default `crate::apply_batch` remains outside this explicit scoped profile.
pub fn apply_batch_with_scope(
    world: &World,
    batch: &CommittedBatch,
    scope: &ReplicaDecodeScope<'_>,
) -> BudgetResult<Result<World, Rejection>> {
    if let Some(error) = scope.failure() {
        return Err(error);
    }
    match scope.nested(|record| replay(world, batch, record)) {
        Ok(next) => Ok(Ok(next)),
        Err(ScopedError::Semantic(error)) => Ok(Err(error)),
        Err(ScopedError::Budget(error)) => Err(error),
    }
}

fn replay(
    world: &World,
    batch: &CommittedBatch,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<World, Rejection> {
    if super::snapshot::hash(world, scope)? != batch.before_hash
        || world.spec.tick.checked_next() != Some(batch.tick)
        || world.spec.event_seq.checked_next() != Some(batch.sequence)
    {
        return Err(ScopedError::Semantic(Rejection::StateMismatch));
    }
    if batch.admitted.len() != batch.outcomes.len() || batch.events.len() > MAX_FRONTIER * 2 {
        return Err(ScopedError::Semantic(Rejection::InvalidFrontier));
    }
    let frontier = super::admit_with_scope(world, &batch.admitted, batch.authority, scope)
        .map_err(ScopedError::Budget)?
        .map_err(ScopedError::Semantic)?;
    if frontier
        .jobs
        .iter()
        .map(|value| &value.intent)
        .ne(batch.admitted.iter())
        || frontier
            .jobs
            .iter()
            .zip(&batch.outcomes)
            .any(|(job, outcome)| {
                job.id() != outcome.job || job.intent.operation != outcome.operation
            })
    {
        return Err(ScopedError::Semantic(Rejection::InvalidFrontier));
    }
    let mut next = super::clone_world_with_scope(world, scope).map_err(ScopedError::Budget)?;
    let mut event_index = 0;
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    for (job, outcome) in frontier.jobs.iter().zip(&batch.outcomes) {
        if outcome.rejection.is_some() {
            continue;
        }
        job.validation.map_err(ScopedError::Semantic)?;
        if job.reads.keys().any(|id| writes.contains(id))
            || job
                .writes
                .keys()
                .any(|id| reads.contains(id) || writes.contains(id))
        {
            return Err(ScopedError::Semantic(Rejection::Conflict));
        }
        let pair = batch
            .events
            .get(event_index..event_index + 2)
            .ok_or(ScopedError::Semantic(Rejection::InvalidProposal))?;
        crate::replay::validate_committed_pair(world, job, pair, batch)
            .map_err(ScopedError::Semantic)?;
        // Existing reducer makes exactly one clone. Reserve that copy; do not clone twice.
        super::world_copy::reserve_world_clone_with_scope(&next, scope)
            .map_err(ScopedError::Budget)?;
        next = crate::reducer::reduce_events(&next, pair).map_err(ScopedError::Semantic)?;
        retain_keys(&mut reads, job.reads.keys().copied(), scope)?;
        retain_keys(&mut writes, job.writes.keys().copied(), scope)?;
        event_index += 2;
    }
    if event_index != batch.events.len() {
        return Err(ScopedError::Semantic(Rejection::InvalidProposal));
    }
    reserve_outcomes(&next, &batch.outcomes, scope)?;
    let next = crate::reducer::finish_tick(next, &batch.outcomes).map_err(ScopedError::Semantic)?;
    if super::snapshot::hash(&next, scope)? != batch.after_hash {
        return Err(ScopedError::Semantic(Rejection::StateMismatch));
    }
    Ok(next)
}

fn retain_keys<I>(
    retained: &mut BTreeSet<AggregateId>,
    keys: I,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection>
where
    I: IntoIterator<Item = AggregateId>,
{
    scope.nested(|collection| {
        for key in keys {
            if retained.contains(&key) {
                continue;
            }
            collection.nested(|row| {
                row.charge_entries(1).map_err(ScopedError::Budget)?;
                row.charge_copied(16).map_err(ScopedError::Budget)?;
                retained.insert(key);
                Ok(())
            })?;
        }
        Ok(())
    })
}

fn reserve_outcomes(
    world: &World,
    outcomes: &[Outcome],
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    // Preserve finish_tick's outcome limit refusal before this new output allocation.
    if world.outcomes.len() + outcomes.len() > MAX_OUTCOMES {
        return Err(ScopedError::Semantic(Rejection::Limit));
    }
    scope.nested(|collection| {
        for _ in outcomes {
            collection.nested(|row| {
                row.charge_entries(1).map_err(ScopedError::Budget)?;
                row.charge_copied(36).map_err(ScopedError::Budget)
            })?;
        }
        Ok(())
    })
}
