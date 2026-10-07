use crate::{Intent, MAX_ENTITIES, MAX_PROVIDERS, Rejection, World, jobs::command_entity};
use alloc::{collections::BTreeSet, vec::Vec};
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedError, ScopedResult};

pub(super) fn deduplicate(
    world: &World,
    intents: &[Intent],
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    scope.nested(|collection| {
        let mut jobs = BTreeSet::new();
        let mut operations = BTreeSet::new();
        let mut requests = BTreeSet::new();
        for intent in intents {
            // Keep the legacy per-intent local-limit/short-circuit duplicate order.
            if intent.expected.len() > MAX_ENTITIES + MAX_PROVIDERS {
                return Err(ScopedError::Semantic(Rejection::Limit));
            }
            if !insert(&mut jobs, intent.job, collection)?
                || !insert(&mut operations, intent.operation, collection)?
                || !insert(&mut requests, intent.request, collection)?
                || world
                    .outcomes
                    .iter()
                    .any(|value| value.operation == intent.operation || value.job == intent.job)
            {
                return Err(ScopedError::Semantic(Rejection::InvalidFrontier));
            }
        }
        Ok(())
    })
}

fn insert<T: Copy + Ord>(
    keys: &mut BTreeSet<T>,
    key: T,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<bool, Rejection> {
    if keys.contains(&key) {
        return Ok(false);
    }
    scope.nested(|row| {
        row.charge_entries(1).map_err(ScopedError::Budget)?;
        row.charge_copied(16).map_err(ScopedError::Budget)?;
        Ok(keys.insert(key))
    })
}

pub(super) fn intents<'a>(
    intents: &'a [Intent],
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<Vec<&'a Intent>, Rejection> {
    scope.nested(|collection| {
        let mut ordered = Vec::new();
        for intent in intents {
            collection.nested(|row| {
                row.charge_entries(1).map_err(ScopedError::Budget)?;
                // Declared logical reference width, independent of machine pointer width.
                row.charge_copied(8).map_err(ScopedError::Budget)?;
                ordered.push(intent);
                Ok(())
            })?;
        }
        // Deduplication proved unique JobIds, hence no equal complete sort keys.
        // The total order matches stable legacy sorting without heap sort scratch.
        ordered.sort_unstable_by_key(|value| {
            (
                value.provider,
                command_entity(&value.command),
                value.operation,
                value.job,
            )
        });
        Ok(ordered)
    })
}
