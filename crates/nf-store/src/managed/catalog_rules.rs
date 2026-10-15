use super::{bytes::{hash, next}, catalog_state::{CatalogState, OwnerIdentity}, challenge::Owned,
    error::{ManagedError, Result}, foot::StorageFoot, maps::Maps, prospective::Prospective,
    stamp::{CatalogFrontier, OwnerStamp}, summary::Summary};

pub(super) fn stamp_owner(state: &CatalogState, stamp: &OwnerStamp) -> Result<()> {
    let foot = state.current.as_ref().ok_or(ManagedError::Corrupt)?;
    if stamp.installation != state.installation || stamp.generation != state.generation
        || stamp.term != state.term || stamp.session != state.session
        || state.owner.as_ref() != Some(&identity(stamp)) || stamp.profile != foot.profile
        || stamp.schema != foot.schema || stamp.membership_revision != foot.known.membership_revision
        || Some(stamp.membership_digest) != foot.member
    { return Err(ManagedError::StaleSession); }
    Ok(())
}
pub(super) fn identity(stamp: &OwnerStamp) -> OwnerIdentity {
    OwnerIdentity { account: stamp.account, device: stamp.device, incarnation: stamp.incarnation }
}
pub(super) fn signed_step(state: &CatalogState, after: &CatalogState, auth: &Owned) -> Result<()> {
    stamp_owner(state, &auth.stamp)?;
    let foot = state.current.as_ref().ok_or(ManagedError::Corrupt)?;
    if auth.sequence != next(state.consumed)? || auth.revision != foot.known.store_revision
        || auth.state != foot.state || auth.head != foot.head || after.utc < auth.times.issued
        || after.utc >= auth.times.expires || after.utc < state.utc
    { return Err(ManagedError::Corrupt); }
    Ok(())
}
pub(super) fn prospective(state: &CatalogState, after: &CatalogState, frontier: CatalogFrontier,
    auth: &Prospective) -> Result<()> {
    if auth.catalog != Some(frontier) || auth.installation != state.installation || auth.scope != state.scope
        || after.utc < state.utc || after.utc < auth.times.issued || after.utc >= auth.times.expires
    { return Err(ManagedError::Corrupt); }
    Ok(())
}
pub(super) fn pending(summary: &Summary, state: &CatalogState) -> Result<()> {
    stamp_owner(state, &summary.stamp)?;
    if summary.artifact != state.selected.unwrap_or(state.source)
        || state.current.as_ref() != Some(&summary.pre) || summary.pre.known.scope != state.scope
        || summary.post.known.scope != state.scope || summary.pre_state != summary.pre.state
        || summary.post_state != summary.post.state
    { return Err(ManagedError::DerivationMismatch); }
    counters(summary)?; summary.effect.check_feet(&summary.pre, &summary.post)
}
fn counters(summary: &Summary) -> Result<()> {
    let before = &summary.pre; let after = &summary.post; let p = summary.purpose;
    let old = before.known; let new = after.known;
    let revision = if before.profile == 4 || matches!(p, 1 | 2 | 3) {
        next(old.store_revision)?
    } else { old.store_revision };
    let membership = if p == 5 { Some(next(old.membership_revision.ok_or(ManagedError::Corrupt)?)?) }
        else { old.membership_revision };
    if new.store_revision != revision || new.membership_revision != membership
        || new.scope != old.scope || (p != 2 && new.event_sequence != old.event_sequence)
        || (p == 2 && new.event_sequence.0 != next(old.event_sequence.0)?)
        || (p == 6 && after.quota >= before.quota) || (p != 6 && after.quota != before.quota)
        || (p != 8 && (before.profile != after.profile || before.schema != after.schema))
        || (p == 8 && (before.profile, after.profile) != (1, 5))
        || before.lineage != after.lineage
    { return Err(ManagedError::DerivationMismatch); }
    if before.profile == 1 {
        if !matches!(p, 1 | 2 | 3 | 5 | 6 | 7 | 8) { return Err(ManagedError::Corrupt); }
        if p == 1 && after.policy.is_none_or(|cut| cut.policy != super::state::PendingPolicy::Drain) {
            return Err(ManagedError::DerivationMismatch);
        }
    }
    if p == 1 && (before.policy.is_some() || after.policy.is_none())
        || p == 2 && (before.policy.is_none() || after.policy.is_some())
        || (!matches!(p, 1 | 2) && before.policy != after.policy)
        || (!matches!(p, 1 | 2) && before.core != after.core)
    { return Err(ManagedError::DerivationMismatch); }
    Ok(())
}
pub(super) fn registry(maps: &Maps, state: &CatalogState) -> Result<()> {
    if maps.registry.keys().copied().collect::<Vec<_>>() != if maps.registry.contains_key(&2) {
        vec![1, 2, 3, 4, 5]
    } else { vec![1, 4, 5] } { return Err(ManagedError::Corrupt); }
    for (kind, row) in &maps.registry {
        let manifest = super::registry::Manifest::decode(&row.manifest)?;
        if manifest.installation != state.installation || manifest.scope != state.scope
            || manifest.kind != *kind || manifest.id != row.artifact || manifest.path != row.path
            || manifest.bytes != row.bytes || manifest.digest != row.digest
        { return Err(ManagedError::Corrupt); }
    }
    if state.source != maps.registry.get(&1).ok_or(ManagedError::Corrupt)?.artifact {
        return Err(ManagedError::Corrupt);
    }
    let (registry, receipts) = maps.digests()?;
    if (registry, receipts) != (state.registry, state.receipts) { return Err(ManagedError::Corrupt); }
    Ok(())
}
pub(super) fn claim_stamp(state: &CatalogState, stamp: &OwnerStamp, foot: &StorageFoot) -> Result<()> {
    if stamp.installation != state.installation || stamp.generation != state.generation
        || stamp.term != next(state.term)? || stamp.session != next(state.session)?
        || stamp.profile != foot.profile || stamp.schema != foot.schema
        || stamp.membership_revision != foot.known.membership_revision || Some(stamp.membership_digest) != foot.member
        || stamp.incarnation == [0; 16]
    { return Err(ManagedError::Corrupt); }
    Ok(())
}
pub(super) fn checkpoint_rows(maps: &Maps) -> Result<[u8; 32]> { Ok(hash(&maps.encode()?)) }
