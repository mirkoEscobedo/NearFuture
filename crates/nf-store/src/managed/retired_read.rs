use super::{bytes::hash, error::{ManagedError, Result}, foot::StorageFoot, mirror_rows::i,
    retirement::Retirement, rows, schema::Profile, state::FullState,
    world_plan::{Plan, fixed32}};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};
pub(super) fn inspect(c: &Connection, expected: &StorageFoot, expected_record: Option<[u8; 32]>)
    -> Result<(FullState, Retirement)> {
    super::schema::physical(c, Profile::Retired)?;
    let records = rows::scan(c, rows::table("owner_retirement")?, 1)?;
    let row = records.first().ok_or(ManagedError::Corrupt)?;
    if row.first() != Some(&i(1)) { return Err(ManagedError::Corrupt); }
    let bytes = rows::blob(row, 1, 8192)?; let digest = hash(bytes);
    let record = Retirement::decode(bytes)?;
    if rows::blob(row, 2, 32)? != digest || expected_record.is_some_and(|sha| sha != digest)
        || record.source_foot.known != expected.known || expected.profile != 5
        || record.source_foot.known.scope != expected.known.scope
        || rows::digest(c, &rows::tables(Profile::Source)?, &BTreeMap::new())? != record.source_foot.rows
    { return Err(ManagedError::SourceMismatch); }
    let (core, revision, head) = crate::recovery::recover(c, record.source_foot.known)?;
    let member = crate::identity::load(c, record.source_foot.known.scope)?.ok_or(ManagedError::Unauthorized)?;
    let state = FullState { core, member: Some(member), quota: record.source_foot.quota,
        policy: None, cancellations: BTreeMap::new() };
    state.validate(revision)?;
    let snapshots = rows::scan(c, rows::table("snapshots")?, 1)?;
    let cp = snapshots.first().ok_or(ManagedError::Corrupt)?;
    let cp_core = crate::codec::decode(rows::blob(cp, 1, 1_048_576)?)?;
    let protected = &record.source_foot.checkpoint;
    if hash(rows::blob(cp, 1, 1_048_576)?) != fixed32(rows::blob(cp, 2, 32)?)?
        || rows::counter(cp, 0)? != protected.revision || cp_core.world.view().event_seq().0 != protected.event
        || rows::blob(cp, 3, 32)? != protected.head || cp_core.scope() != expected.known.scope
    { return Err(ManagedError::SourceMismatch); }
    let actual = Plan { state: state.clone(), revision, head, checkpoint: protected.clone(), patches: BTreeMap::new() }
        .foot(c, Profile::Retired, None, &BTreeSet::new())?;
    if &actual != expected { return Err(ManagedError::SourceMismatch); }
    Ok((state, record))
}
