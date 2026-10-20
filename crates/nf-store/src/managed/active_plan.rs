use super::{
    bytes::{hash, next},
    error::{ManagedError, Result},
    foot::StorageFoot,
    mirror_rows::{b, i, u},
    rows::{self, Patch, Row},
    state::FullState,
    upgrade,
    world_plan::{Plan, new_checkpoint},
};
use nf_kernel::CommittedBatch;
use rusqlite::{Connection, params};
use std::collections::BTreeMap;

pub(super) fn initialize(c: &Connection, initial: &upgrade::Initial) -> Result<()> {
    for ddl in super::schema::ACTIVE_SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let name = ddl
            .strip_prefix("CREATE TABLE ")
            .and_then(|s| s.split_whitespace().next())
            .ok_or(ManagedError::Corrupt)?;
        if name.starts_with("active_") || name == "migration_lineage" {
            c.execute_batch(ddl)?;
        }
    }
    c.pragma_update(None, "user_version", 4)?;
    let state = initial.state.encode()?;
    let digest = hash(&state);
    let lineage = hash(&initial.lineage);
    let member = initial
        .state
        .member
        .as_ref()
        .map(|m| m.revision.to_be_bytes());
    let revision = initial.revision.to_be_bytes();
    let event = initial.state.core.world.view().event_seq().0.to_be_bytes();
    let quota = initial.state.quota.to_le_bytes();
    c.execute(
        "INSERT INTO migration_lineage VALUES(1,?1,?2)",
        params![initial.lineage, lineage],
    )?;
    c.execute(
        "INSERT INTO active_journal VALUES(?1,4,?2,?3,?4,?5,?6)",
        params![
            revision,
            initial.before,
            digest,
            initial.head,
            initial.action,
            state
        ],
    )?;
    c.execute(
        "INSERT INTO active_snapshots VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            revision,
            event,
            member,
            quota,
            state,
            digest,
            initial.head,
            lineage
        ],
    )?;
    c.execute(
        "INSERT INTO active_meta VALUES(1,?1,?2,?3,?4,?5,?6,?7,?8,?9,?1,?6,?7)",
        params![
            revision,
            event,
            member,
            quota,
            state,
            digest,
            initial.head,
            super::schema::active_schema_digest(),
            lineage
        ],
    )?;
    Ok(())
}
pub(super) fn initial_row(c: &Connection) -> Result<Row> {
    let table = rows::table("active_journal")?;
    let rows = rows::scan(c, table, 257)?;
    let mut selected = rows.into_iter().filter(|row| row.get(1) == Some(&i(4)));
    let row = selected.next().ok_or(ManagedError::Corrupt)?;
    if selected.next().is_some() {
        return Err(ManagedError::Corrupt);
    }
    Ok(row)
}
pub(super) fn plan(
    c: &Connection,
    before: &StorageFoot,
    state: FullState,
    purpose: u32,
    action: &[u8],
    batch: Option<&CommittedBatch>,
) -> Result<Plan> {
    if before.profile != 4 || !matches!(purpose, 1 | 2 | 3 | 5 | 6 | 7) {
        return Err(ManagedError::UnsupportedFormat);
    }
    let revision = next(before.known.store_revision)?;
    state.validate(revision)?;
    let envelope = state.encode()?;
    let digest = hash(&envelope);
    let lineage = before.lineage.ok_or(ManagedError::Corrupt)?;
    let head = upgrade::head(
        before.head,
        revision,
        purpose,
        action,
        before.state,
        digest,
        lineage,
    )?;
    let mut checkpoint = before.checkpoint.clone();
    let mut patches = BTreeMap::new();
    let checkpointed = purpose == 7
        || revision
            .checked_sub(checkpoint.revision)
            .ok_or(ManagedError::Corrupt)?
            >= 32;
    let initial = initial_row(c)?;
    let initial_revision = rows::counter(&initial, 0)?;
    let journal = rows::table("active_journal")?;
    let mut journal_patch = Patch::default();
    if checkpointed {
        for row in rows::scan(c, journal, 257)? {
            if rows::counter(&row, 0)? != initial_revision {
                journal_patch.put(journal, Some(row), None)?;
            }
        }
    } else {
        journal_patch.put(
            journal,
            None,
            Some(vec![
                u(revision),
                i(i64::from(purpose)),
                b(before.state),
                b(digest),
                b(head),
                b(action),
                b(&envelope),
            ]),
        )?;
    }
    if !journal_patch.0.is_empty() {
        patches.insert(journal.name.to_string(), journal_patch);
    }
    let events = rows::table("active_events")?;
    let mut events_patch = Patch::default();
    if checkpointed {
        for row in rows::scan(c, events, 256)? {
            events_patch.put(events, Some(row), None)?;
        }
    } else if let Some(batch) = batch {
        let bytes = nf_kernel::encode_batch(batch)?;
        events_patch.put(
            events,
            None,
            Some(vec![
                u(batch.sequence.0),
                u(revision),
                b(&bytes),
                b(hash(&bytes)),
            ]),
        )?;
    }
    if !events_patch.0.is_empty() {
        patches.insert(events.name.to_string(), events_patch);
    }
    if checkpointed {
        checkpoint = new_checkpoint(&state, revision, head)?;
        let snapshots = rows::table("active_snapshots")?;
        let mut patch = Patch::default();
        for row in rows::scan(c, snapshots, 2)? {
            if rows::counter(&row, 0)? != initial_revision {
                patch.put(snapshots, Some(row), None)?;
            }
        }
        patch.put(
            snapshots,
            None,
            Some(snapshot(&state, revision, head, lineage)?),
        )?;
        patches.insert(snapshots.name.to_string(), patch);
    }
    let table = rows::table("active_meta")?;
    let old = rows::get(c, table, &vec![i(1)])?.ok_or(ManagedError::Corrupt)?;
    let mut new = old.clone();
    new[1] = u(revision);
    new[2] = u(state.core.world.view().event_seq().0);
    new[3] = state
        .member
        .as_ref()
        .map_or(super::rows::Cell::Null, |m| u(m.revision));
    new[4] = b(state.quota.to_le_bytes());
    new[5] = b(&envelope);
    new[6] = b(digest);
    new[7] = b(head);
    new[10] = u(checkpoint.revision);
    new[11] = b(checkpoint.state);
    new[12] = b(checkpoint.head);
    let mut patch = Patch::default();
    patch.put(table, Some(old), Some(new))?;
    patches.insert(table.name.to_string(), patch);
    Ok(Plan {
        state,
        revision,
        head,
        checkpoint,
        patches,
    })
}
pub(super) fn snapshot(
    state: &FullState,
    revision: u64,
    head: [u8; 32],
    lineage: [u8; 32],
) -> Result<Row> {
    let bytes = state.encode()?;
    Ok(vec![
        u(revision),
        u(state.core.world.view().event_seq().0),
        state
            .member
            .as_ref()
            .map_or(super::rows::Cell::Null, |m| u(m.revision)),
        b(state.quota.to_le_bytes()),
        b(&bytes),
        b(hash(&bytes)),
        b(head),
        b(lineage),
    ])
}
