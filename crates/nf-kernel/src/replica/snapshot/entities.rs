use super::{ordering, writer::Writer};
use crate::{Rejection, World};
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedResult};

pub(super) fn write(
    world: &World,
    writer: &mut Writer,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    let spec = &world.spec;
    scope.nested(|collection| {
        let items = ordering::rows(&spec.factions, |value| value.id, collection)?;
        writer.count(items.len(), collection)?;
        for value in items {
            collection.nested(|row| {
                writer.fixed(value.id.as_bytes(), row)?;
                writer.fixed(value.aggregate.as_bytes(), row)
            })?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        let items = ordering::rows(&spec.relations, |value| value.id, collection)?;
        writer.count(items.len(), collection)?;
        for value in items {
            collection.nested(|row| {
                writer.fixed(value.id.as_bytes(), row)?;
                writer.fixed(value.aggregate.as_bytes(), row)?;
                writer.fixed(value.left.as_bytes(), row)?;
                writer.fixed(value.right.as_bytes(), row)?;
                writer.i64(i64::from(value.score), row)
            })?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        let items = ordering::rows(&spec.markets, |value| value.id, collection)?;
        writer.count(items.len(), collection)?;
        for value in items {
            collection.nested(|row| {
                writer.fixed(value.id.as_bytes(), row)?;
                writer.fixed(value.aggregate.as_bytes(), row)?;
                writer.fixed(value.faction.as_bytes(), row)?;
                writer.u64(value.credits, row)
            })?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        let items = ordering::rows(&spec.providers, |value| value.id, collection)?;
        writer.count(items.len(), collection)?;
        for value in items {
            collection.nested(|row| {
                writer.fixed(value.id.as_bytes(), row)?;
                writer.fixed(value.aggregate.as_bytes(), row)?;
                writer.u64(value.draws, row)?;
                writer.u64(value.cooldown_until.0, row)
            })?;
        }
        Ok(())
    })
}
