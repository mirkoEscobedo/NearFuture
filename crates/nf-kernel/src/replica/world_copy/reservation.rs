use crate::World;
use core::convert::Infallible;
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedError, ScopedResult};

const WORLD_FIXED_BYTES: u64 = 112;
const FACTION_BYTES: u64 = 32;
const RELATION_BYTES: u64 = 68;
const MARKET_BYTES: u64 = 56;
const PROVIDER_BYTES: u64 = 48;
const MANIFEST_FIXED_BYTES: u64 = 64;
const OWNERSHIP_BYTES: u64 = 80;
const REVISION_ROW_BYTES: u64 = 24;
const CLOSED_ENUM_BYTES: u64 = 4;
const PRINCIPAL_BYTES: u64 = 16;
const OUTCOME_BYTES: u64 = 36;

pub(super) fn world(world: &World, scope: &ReplicaDecodeScope<'_>) -> ScopedResult<(), Infallible> {
    // The borrowed valid input remains owned elsewhere; reserve only the new copy.
    // Vec/set/map headers and allocator capacity are outside the logical payload model.
    scope
        .charge_copied(WORLD_FIXED_BYTES)
        .map_err(ScopedError::Budget)?;
    let spec = &world.spec;
    fixed_rows(spec.factions.iter(), FACTION_BYTES, scope)?;
    fixed_rows(spec.relations.iter(), RELATION_BYTES, scope)?;
    fixed_rows(spec.markets.iter(), MARKET_BYTES, scope)?;
    fixed_rows(spec.providers.iter(), PROVIDER_BYTES, scope)?;
    manifests(world, scope)?;
    fixed_rows(spec.ownership.iter(), OWNERSHIP_BYTES, scope)?;
    // Each cloned map row separately retains its AggregateId key and revision value.
    fixed_rows(spec.revisions.iter(), REVISION_ROW_BYTES, scope)?;
    fixed_rows(world.outcomes.iter(), OUTCOME_BYTES, scope)
}

fn manifests(world: &World, scope: &ReplicaDecodeScope<'_>) -> ScopedResult<(), Infallible> {
    scope.nested(|collection| {
        for manifest in &world.spec.registry {
            collection.nested(|row| {
                row.charge_entries(1).map_err(ScopedError::Budget)?;
                row.charge_copied(MANIFEST_FIXED_BYTES)
                    .map_err(ScopedError::Budget)?;
                fixed_rows(manifest.read_domains.iter(), CLOSED_ENUM_BYTES, row)?;
                fixed_rows(manifest.write_domains.iter(), CLOSED_ENUM_BYTES, row)?;
                fixed_rows(manifest.capabilities.iter(), CLOSED_ENUM_BYTES, row)?;
                fixed_rows(manifest.principals.iter(), PRINCIPAL_BYTES, row)
            })?;
        }
        Ok(())
    })
}

fn fixed_rows<I>(
    values: I,
    bytes: u64,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Infallible>
where
    I: IntoIterator,
{
    scope.nested(|collection| {
        for _ in values {
            collection.nested(|row| {
                row.charge_entries(1).map_err(ScopedError::Budget)?;
                row.charge_copied(bytes).map_err(ScopedError::Budget)
            })?;
        }
        Ok(())
    })
}
