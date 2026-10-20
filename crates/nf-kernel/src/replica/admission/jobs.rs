use crate::{Command, Intent, Job, Rejection, World};
use alloc::{collections::BTreeMap, vec::Vec};
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedError, ScopedResult};

pub(super) fn build(
    world: &World,
    intents: Vec<&Intent>,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<Vec<Job>, Rejection> {
    scope.nested(|collection| {
        let mut jobs = Vec::new();
        for intent in intents {
            collection.nested(|row| {
                row.charge_entries(1).map_err(ScopedError::Budget)?;
                row.charge_copied(4).map_err(ScopedError::Budget)?;
                let access = super::super::provider::validate_intent_with_scope(world, intent, row)
                    .map_err(ScopedError::Budget)?;
                let (reads, writes, validation) = match access {
                    Ok((reads, writes)) => (reads, writes, Ok(())),
                    Err(error) => (BTreeMap::new(), BTreeMap::new(), Err(error)),
                };
                reserve_intent(intent, row)?;
                // Existing charged Access maps move intact; only the Intent is copied.
                jobs.push(Job {
                    intent: intent.clone(),
                    reads,
                    writes,
                    validation,
                });
                Ok(())
            })?;
        }
        Ok(jobs)
    })
}

fn reserve_intent(intent: &Intent, scope: &ReplicaDecodeScope<'_>) -> ScopedResult<(), Rejection> {
    scope.nested(|record| {
        record.charge_copied(112).map_err(ScopedError::Budget)?;
        let command_bytes = match intent.command {
            Command::SeekPeace { .. } => 20,
            Command::AdjustRelation { .. } => 24,
            Command::AdjustMarket { .. } => 28,
        };
        record
            .charge_copied(command_bytes)
            .map_err(ScopedError::Budget)?;
        record.nested(|collection| {
            for _ in &intent.expected {
                collection.nested(|row| {
                    row.charge_entries(1).map_err(ScopedError::Budget)?;
                    // Each new map row retains an independent16-byte key and8-byte value.
                    row.charge_copied(24).map_err(ScopedError::Budget)
                })?;
            }
            Ok(())
        })
    })
}
