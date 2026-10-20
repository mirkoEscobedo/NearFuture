use super::{ordering, writer::Writer};
use crate::{ProviderManifest, Rejection, World};
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedResult};

pub(super) fn write(
    world: &World,
    writer: &mut Writer,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    scope.nested(|collection| {
        let items = ordering::rows(&world.spec.registry, |value| value.id, collection)?;
        writer.count(items.len(), collection)?;
        for manifest in items {
            collection.nested(|row| {
                writer.fixed(manifest.id.as_bytes(), row)?;
                writer.fixed(&manifest.implementation_hash, row)?;
                writer.u32(manifest.version, row)?;
                writer.u32(manifest.state_schema, row)?;
                writer.u32(manifest.kind as u32 + 1, row)?;
                sets(manifest, writer, row)?;
                writer.u32(manifest.max_events, row)
            })?;
        }
        Ok(())
    })
}

fn sets(
    manifest: &ProviderManifest,
    writer: &mut Writer,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    scope.nested(|collection| {
        writer.count(manifest.read_domains.len(), collection)?;
        for value in &manifest.read_domains {
            collection.nested(|row| writer.u32(*value as u32 + 1, row))?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        writer.count(manifest.write_domains.len(), collection)?;
        for value in &manifest.write_domains {
            collection.nested(|row| writer.u32(*value as u32 + 1, row))?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        writer.count(manifest.capabilities.len(), collection)?;
        for value in &manifest.capabilities {
            collection.nested(|row| writer.u32(*value as u32 + 1, row))?;
        }
        Ok(())
    })?;
    scope.nested(|collection| {
        writer.count(manifest.principals.len(), collection)?;
        for value in &manifest.principals {
            collection.nested(|row| writer.fixed(value.as_bytes(), row))?;
        }
        Ok(())
    })
}
