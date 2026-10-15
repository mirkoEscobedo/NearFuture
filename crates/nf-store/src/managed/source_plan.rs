use super::{bytes::{hash, next}, error::{ManagedError, Result}, foot::StorageFoot,
    mirror_rows::{self, b, i, u}, mutation::Mutation, rows::{self, Patch},
    schema::Profile, state::{FullState, PendingCut}, world_plan::{Plan, fixed32, new_checkpoint}};
use crate::Store;
use nf_identity::model::MembershipState;
use nf_kernel::CommittedBatch;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn inspect(store: &Store, member: MembershipState, quota: u32, policy: Option<PendingCut>,
    protected: Option<&StorageFoot>) -> Result<(FullState, StorageFoot)> {
    let known = store.known_frontiers()?; let head = store.verified_source_head(known)?;
    let (state, foot) = inspect_connection(&store.connection, known, quota, policy, protected)?;
    if state.core != store.state || foot.head != head || state.member.as_ref() != Some(&member)
        || foot.known.store_revision != store.revision() { return Err(ManagedError::SourceMismatch); }
    Ok((state, foot))
}
pub(super) fn inspect_connection(c: &Connection, minimum: crate::KnownFrontiers, quota: u32,
    policy: Option<PendingCut>, protected: Option<&StorageFoot>) -> Result<(FullState, StorageFoot)> {
    super::schema::physical(c, Profile::Source)?;
    let (core, revision, head) = crate::recovery::recover(c, minimum)?;
    let member = crate::identity::load(c, minimum.scope)?.ok_or(ManagedError::Unauthorized)?;
    let state = FullState { core, member: Some(member.clone()), quota, policy, cancellations: BTreeMap::new() };
    state.validate(revision)?;
    let snapshot = rows::scan(c, rows::table("snapshots")?, 1)?;
    let cp = snapshot.first().ok_or(ManagedError::Corrupt)?;
    let cp_core = crate::codec::decode(rows::blob(cp, 1, 1_048_576)?)?;
    if hash(rows::blob(cp, 1, 1_048_576)?) != fixed32(rows::blob(cp, 2, 32)?)?
        || cp_core.scope() != minimum.scope { return Err(ManagedError::Corrupt); }
    let cp_revision = rows::counter(cp, 0)?; let cp_event = cp_core.world.view().event_seq().0;
    let cp_head = fixed32(rows::blob(cp, 3, 32)?)?;
    let checkpoint = if let Some(protected) = protected {
        if protected.profile != 1 || protected.checkpoint.revision != cp_revision
            || protected.checkpoint.event != cp_event || protected.checkpoint.head != cp_head
        { return Err(ManagedError::SourceMismatch); }
        protected.checkpoint.clone()
    } else {
        if cp_core.pending.is_some() { return Err(ManagedError::NotDrained); }
        let projected = FullState { core: cp_core, member: Some(member), quota, policy: None,
            cancellations: BTreeMap::new() };
        new_checkpoint(&projected, cp_revision, cp_head)?
    };
    let foot = Plan { state: state.clone(), revision, head, checkpoint, patches: BTreeMap::new() }
        .foot(c, Profile::Source, None, &BTreeSet::new())?;
    if protected.is_some_and(|expected| expected != &foot) { return Err(ManagedError::SourceMismatch); }
    Ok((state, foot))
}
pub(super) fn plan(c: &Connection, before: &FullState, foot: &StorageFoot, after: FullState,
    mutation: &Mutation, batch: Option<&CommittedBatch>) -> Result<Plan> {
    if foot.profile != 1 { return Err(ManagedError::UnsupportedFormat); }
    let ordinary = matches!(mutation, Mutation::Prepare(_) | Mutation::Settlement { .. } | Mutation::Drain
        | Mutation::Delivered(_));
    let revision = if ordinary { next(foot.known.store_revision)? } else { foot.known.store_revision };
    let core = crate::codec::encode(&after.core)?; let digest = hash(&core);
    let mut patches = if ordinary { mirror_rows::diff(&before.core, &after.core)? } else { BTreeMap::new() };
    let (kind, action) = match mutation {
        Mutation::Prepare(value) => (1, nf_kernel::encode_frontier(&value.frontier)?),
        Mutation::Settlement { .. } | Mutation::Drain => (2, nf_kernel::encode_batch(batch.ok_or(ManagedError::Corrupt)?)?),
        Mutation::Delivered(operation) => (3, operation.as_bytes().to_vec()),
        Mutation::Membership { .. } | Mutation::Quota(_) | Mutation::Compact => (0, Vec::new()),
        Mutation::Cancel { .. } => return Err(ManagedError::OperationConflict),
    };
    let head = if ordinary { crate::persistence::head(foot.head, revision, kind, &action, digest) } else { foot.head };
    let checkpointed = matches!(mutation, Mutation::Compact) || (ordinary && revision % 128 == 0);
    if ordinary {
        let table = rows::table("history_meta")?;
        let old = rows::get(c, table, &vec![i(1)])?.ok_or(ManagedError::Corrupt)?;
        let mut new = old.clone(); new[3] = u(revision); new[4] = u(after.core.world.view().event_seq().0);
        new[5] = b(&core); new[6] = b(digest); new[7] = b(head);
        let mut patch = Patch::default(); patch.put(table, Some(old), Some(new))?;
        patches.insert(table.name.to_string(), patch);
        if !checkpointed {
            let table = rows::table("journal")?;
            let row = vec![u(revision), i(i64::from(kind)), b(foot.core), b(digest), b(head), b(&action), b(&core)];
            let mut patch = Patch::default(); patch.put(table, None, Some(row))?;
            patches.insert(table.name.to_string(), patch);
            if let Some(batch) = batch {
                let table = rows::table("events")?;
                let mut patch = Patch::default(); patch.put(table, None,
                    Some(vec![u(batch.sequence.0), b(&action), b(hash(&action))]))?;
                patches.insert(table.name.to_string(), patch);
            }
        }
    }
    if matches!(mutation, Mutation::Membership { .. }) {
        let member = after.member.as_ref().ok_or(ManagedError::Unauthorized)?;
        let encoded = nf_identity::codec::encode_state(member)?;
        let table = rows::table("membership")?;
        let key = vec![b(member.scope.universe.as_bytes()), b(member.scope.history.as_bytes())];
        let old = rows::get(c, table, &key)?.ok_or(ManagedError::Corrupt)?;
        let mut patch = Patch::default(); patch.put(table, Some(old), Some(vec![key[0].clone(), key[1].clone(),
            u(member.revision), b(&encoded), b(hash(&encoded))]))?;
        patches.insert(table.name.to_string(), patch);
    }
    let mut checkpoint = foot.checkpoint.clone();
    if checkpointed {
        let table = rows::table("snapshots")?;
        let old = rows::scan(c, table, 1)?;
        let new = vec![u(revision), b(&core), b(digest), b(head)];
        let mut patch = Patch::default(); let mut changed = false;
        for row in old {
            if rows::counter(&row, 0)? == revision {
                if row != new { return Err(ManagedError::DerivationMismatch); }
            } else { patch.put(table, Some(row), None)?; changed = true; }
        }
        if rows::get(c, table, &vec![u(revision)])?.is_none() {
            patch.put(table, None, Some(new))?; changed = true;
        }
        if !patch.0.is_empty() { patches.insert(table.name.to_string(), patch); }
        if changed { checkpoint = new_checkpoint(&after, revision, head)?; }
        prune(c, &mut patches, "journal", revision)?;
        prune(c, &mut patches, "events", after.core.world.view().event_seq().0)?;
    }
    Ok(Plan { state: after, revision, head, checkpoint, patches })
}
fn prune(c: &Connection, patches: &mut BTreeMap<String, Patch>, name: &str, cutoff: u64) -> Result<()> {
    let table = rows::table(name)?; let mut patch = Patch::default();
    for row in rows::scan(c, table, 256)? {
        if rows::counter(&row, 0)? <= cutoff { patch.put(table, Some(row), None)?; }
    }
    if !patch.0.is_empty() { patches.insert(name.to_string(), patch); }
    Ok(())
}
