use super::{ordering, writer::Writer};
use crate::{Rejection, World};
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedResult};

pub(super) fn fixed(
    world: &World,
    writer: &mut Writer,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    let spec = &world.spec;
    writer.fixed(spec.universe.as_bytes(), scope)?;
    writer.fixed(spec.history.as_bytes(), scope)?;
    writer.fixed(&spec.seed, scope)?;
    writer.fixed(&spec.ruleset_hash, scope)?;
    writer.u64(spec.tick.0, scope)?;
    writer.u64(spec.event_seq.0, scope)
}

pub(super) fn collections(
    world: &World,
    writer: &mut Writer,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    scope.nested(|collection| {
        let items = ordering::rows(&world.spec.ownership, |value| value.aggregate, collection)?;
        writer.count(items.len(), collection)?;
        for value in items {
            collection.nested(|row| {
                writer.fixed(value.aggregate.as_bytes(), row)?;
                writer.fixed(value.provider.as_bytes(), row)?;
                writer.u64(value.generation, row)?;
                writer.u64(value.activation_seq.0, row)?;
                writer.fixed(&value.ruleset_hash, row)
            })?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        writer.count(world.spec.revisions.len(), collection)?;
        for (id, revision) in &world.spec.revisions {
            collection.nested(|row| {
                writer.fixed(id.as_bytes(), row)?;
                writer.u64(revision.0, row)
            })?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        writer.count(world.outcomes.len(), collection)?;
        for outcome in &world.outcomes {
            collection.nested(|row| {
                writer.fixed(outcome.operation.as_bytes(), row)?;
                writer.fixed(outcome.job.as_bytes(), row)?;
                writer.u32(outcome.rejection.map_or(0, |error| error as u32 + 1), row)
            })?;
        }
        Ok(())
    })
}
