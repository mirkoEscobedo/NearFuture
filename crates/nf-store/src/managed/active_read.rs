use super::{active_origin::Origin, active_plan, auth, bytes::hash, error::{ManagedError, Result},
    foot::StorageFoot, mirror_rows::{b, u}, mutation::{self, Derived, Mutation},
    rows::{self, Cell, Row}, schema::Profile, state::FullState, upgrade,
    world_plan::{Plan, fixed32, new_checkpoint}};
use crate::KnownFrontiers;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Recovered {
    pub state: FullState,
    pub foot: StorageFoot,
}
pub(super) fn inspect(c: &Connection, minimum: KnownFrontiers, protected: Option<&StorageFoot>) -> Result<Recovered> {
    super::schema::physical(c, Profile::Active)?;
    let origin = Origin::read(c)?;
    if active_plan::initial_row(c)? != origin.journal()? { return Err(ManagedError::DerivationMismatch); }
    let snapshots = rows::scan(c, rows::table("active_snapshots")?, 2)?;
    let initial_snapshot = active_plan::snapshot(&origin.state, origin.revision, origin.head, origin.lineage)?;
    if !snapshots.iter().any(|row| row == &initial_snapshot) { return Err(ManagedError::Corrupt); }
    let meta = rows::get(c, rows::table("active_meta")?, &vec![Cell::Integer(1)])?.ok_or(ManagedError::Corrupt)?;
    let revision = rows::counter(&meta, 1)?; let digest = fixed32(rows::blob(&meta, 6, 32)?)?;
    let head = fixed32(rows::blob(&meta, 7, 32)?)?;
    let current_bytes = rows::blob(&meta, 5, 1_884_160)?;
    let current = FullState::decode(current_bytes, revision)?;
    if hash(current_bytes) != digest || rows::counter(&meta, 2)? != current.core.world.view().event_seq().0
        || optional_counter(&meta, 3)? != current.member.as_ref().map(|m| m.revision)
        || le_quota(&meta, 4)? != current.quota || rows::blob(&meta, 8, 32)? != super::schema::active_schema_digest()
        || rows::blob(&meta, 9, 32)? != origin.lineage || current.core.scope() != minimum.scope
        || current.core.world.to_spec().ruleset_hash != origin.to.digest()
    { return Err(ManagedError::Corrupt); }
    let cp_revision = rows::counter(&meta, 10)?;
    let cp = snapshots.iter().find(|row| rows::counter(row, 0).is_ok_and(|r| r == cp_revision)).ok_or(ManagedError::Corrupt)?;
    if cp_revision > revision || cp_revision < origin.revision || revision - cp_revision >= 32
        || snapshots.iter().any(|row| rows::counter(row, 0).is_ok_and(|r| r != origin.revision && r != cp_revision))
    { return Err(ManagedError::Corrupt); }
    let cp_bytes = rows::blob(cp, 4, 1_884_160)?;
    let mut state = FullState::decode(cp_bytes, cp_revision)?;
    let cp_head = fixed32(rows::blob(cp, 6, 32)?)?;
    let checkpoint = new_checkpoint(&state, cp_revision, cp_head)?;
    if rows::counter(cp, 1)? != checkpoint.event || optional_counter(cp, 2)? != checkpoint.membership
        || le_quota(cp, 3)? != checkpoint.quota || rows::blob(cp, 5, 32)? != checkpoint.state
        || rows::blob(cp, 7, 32)? != origin.lineage || rows::blob(&meta, 11, 32)? != checkpoint.state
        || rows::blob(&meta, 12, 32)? != cp_head
    { return Err(ManagedError::Corrupt); }
    if cp_revision == origin.revision {
        if cp != &initial_snapshot { return Err(ManagedError::DerivationMismatch); }
    } else if protected.is_none_or(|foot| foot.profile != 4 || foot.checkpoint != checkpoint) {
        return Err(ManagedError::StaleBackup);
    }
    let mut expected_revision = cp_revision; let mut expected_head = cp_head; let mut events = Vec::new();
    let journal = rows::scan(c, rows::table("active_journal")?, 257)?;
    for row in &journal {
        let r = rows::counter(row, 0)?;
        if r == origin.revision { continue; }
        if r <= cp_revision { return Err(ManagedError::Corrupt); }
        if expected_revision.checked_add(1) != Some(r) { return Err(ManagedError::Corrupt); }
        let kind = integer(row, 1)?;
        let action = rows::blob(row, 5, 1_114_112)?;
        let candidate_bytes = rows::blob(row, 6, 1_884_160)?;
        let candidate = FullState::decode(candidate_bytes, r)?;
        let (owned, payload, proof) = auth::decode_action(action)?;
        if owned.purpose != kind { return Err(ManagedError::Corrupt); }
        let change = Mutation::decode(kind, &payload)?;
        mutation::verify_owner(&state, &owned, &proof, change.role())?;
        let result = change.derive_at(&state, 4, expected_revision, expected_head, &owned)?;
        let (derived, batch) = match result {
            Derived::Paused(_) => return Err(ManagedError::DerivationMismatch),
            Derived::Applied { state, batch, .. } => (state, batch),
        };
        if candidate != derived || rows::blob(row, 2, 32)? != hash(&state.encode()?)
            || rows::blob(row, 3, 32)? != hash(candidate_bytes)
        { return Err(ManagedError::DerivationMismatch); }
        let new_head = upgrade::head(expected_head, r, kind, action, hash(&state.encode()?),
            hash(candidate_bytes), origin.lineage)?;
        if rows::blob(row, 4, 32)? != new_head { return Err(ManagedError::Corrupt); }
        if let Some(batch) = batch {
            let body = nf_kernel::encode_batch(&batch)?;
            events.push(vec![u(batch.sequence.0), u(r), b(&body), b(hash(&body))]);
        }
        state = candidate; expected_revision = r; expected_head = new_head;
    }
    if expected_revision != revision || expected_head != head || state != current {
        return Err(ManagedError::DerivationMismatch);
    }
    let actual_events = rows::scan(c, rows::table("active_events")?, 256)?;
    if actual_events != events { return Err(ManagedError::DerivationMismatch); }
    let foot = Plan { state: current.clone(), revision, head, checkpoint, patches: BTreeMap::new() }
        .foot(c, Profile::Active, Some(origin.lineage), &BTreeSet::new())?;
    if foot.known.scope != minimum.scope || foot.known.store_revision < minimum.store_revision
        || foot.known.event_sequence < minimum.event_sequence || foot.known.membership_revision < minimum.membership_revision
        || protected.is_some_and(|expected| expected != &foot)
    { return Err(ManagedError::StaleBackup); }
    Ok(Recovered { state: current, foot })
}
fn integer(row: &Row, at: usize) -> Result<u32> {
    match row.get(at) { Some(Cell::Integer(n)) => (*n).try_into().map_err(|_| ManagedError::Corrupt),
        _ => Err(ManagedError::Corrupt) }
}
fn optional_counter(row: &Row, at: usize) -> Result<Option<u64>> {
    match row.get(at) { Some(Cell::Null) => Ok(None), Some(Cell::Blob(_)) => Ok(Some(rows::counter(row, at)?)),
        _ => Err(ManagedError::Corrupt) }
}
fn le_quota(row: &Row, at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(rows::blob(row, at, 4)?.try_into().map_err(|_| ManagedError::Corrupt)?))
}
