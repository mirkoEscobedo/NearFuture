use crate::{Rejection, World};
use alloc::collections::{BTreeMap, BTreeSet};
use nf_contract::canonical::replica_budget::{ReplicaDecodeScope, ScopedError, ScopedResult};
use nf_contract::identity::{AggregateId, AggregateRevision, EntityId};

const READ_ID_BYTES: u64 = 16;
const ACCESS_ROW_BYTES: u64 = 24;

pub(super) fn read_ids(
    world: &World,
    target: AggregateId,
    provider: AggregateId,
    factions: &[EntityId],
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<BTreeSet<AggregateId>, Rejection> {
    scope.nested(|collection| {
        let mut ids = BTreeSet::new();
        for id in [target, provider] {
            insert_id(&mut ids, id, collection)?;
        }
        for faction in factions {
            let id = world
                .spec
                .factions
                .iter()
                .find(|value| value.id == *faction)
                .ok_or(ScopedError::Semantic(Rejection::InvalidReference))?
                .aggregate;
            insert_id(&mut ids, id, collection)?;
        }
        Ok(ids)
    })
}
fn insert_id(
    ids: &mut BTreeSet<AggregateId>,
    id: AggregateId,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    if ids.contains(&id) {
        return Ok(());
    }
    scope.nested(|row| {
        row.charge_entries(1).map_err(ScopedError::Budget)?;
        row.charge_copied(READ_ID_BYTES)
            .map_err(ScopedError::Budget)?;
        ids.insert(id);
        Ok(())
    })
}
pub(super) fn reads(
    world: &World,
    ids: BTreeSet<AggregateId>,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<BTreeMap<AggregateId, AggregateRevision>, Rejection> {
    scope.nested(|collection| {
        let mut reads = BTreeMap::new();
        for id in ids {
            let revision = world
                .view()
                .revision(id)
                .ok_or(ScopedError::Semantic(Rejection::InvalidReference))?;
            insert_revision(&mut reads, id, revision, collection)?;
        }
        Ok(reads)
    })
}
pub(super) fn writes(
    target: AggregateId,
    provider: AggregateId,
    reads: &BTreeMap<AggregateId, AggregateRevision>,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<BTreeMap<AggregateId, AggregateRevision>, Rejection> {
    scope.nested(|collection| {
        let mut writes = BTreeMap::new();
        for (id, revision) in [(target, reads[&target]), (provider, reads[&provider])] {
            insert_revision(&mut writes, id, revision, collection)?;
        }
        Ok(writes)
    })
}
fn insert_revision(
    entries: &mut BTreeMap<AggregateId, AggregateRevision>,
    id: AggregateId,
    revision: AggregateRevision,
    scope: &ReplicaDecodeScope<'_>,
) -> ScopedResult<(), Rejection> {
    if let Some(existing) = entries.get_mut(&id) {
        *existing = revision;
        return Ok(());
    }
    scope.nested(|row| {
        // Fresh logical map-row admission; consumed read-ID storage is not refunded.
        // The fixed charges use the primitive's checked cumulative additions.
        row.charge_entries(1).map_err(ScopedError::Budget)?;
        row.charge_copied(ACCESS_ROW_BYTES)
            .map_err(ScopedError::Budget)?;
        entries.insert(id, revision);
        Ok(())
    })
}
