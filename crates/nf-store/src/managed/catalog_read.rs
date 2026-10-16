use super::{
    bytes::hash,
    catalog_action::Action,
    catalog_reduce::ReplayGuard,
    catalog_rules,
    catalog_state::{self, CatalogState},
    error::{ManagedError, Result},
    maps::{Maps, ReceiptRow},
    pending::Pending,
    registry::{Manifest, RegistryRow},
    rows::{self, Cell, Row},
    schema::Profile,
    stamp::CatalogFrontier,
    summary::Summary,
    world_plan::fixed32,
};
use rusqlite::Connection;
use std::{collections::BTreeMap, path::PathBuf};

pub(super) struct Loaded {
    pub state: CatalogState,
    pub frontier: CatalogFrontier,
    pub maps: Maps,
    pub guard: ReplayGuard,
    pub checkpoint: u64,
    pub pending: Option<Pending>,
}
pub(super) fn maps(c: &Connection) -> Result<Maps> {
    let mut result = Maps::default();
    for row in rows::scan(c, rows::table("artifact_registry")?, 5)? {
        let kind = integer(&row, 0)?;
        let manifest = rows::blob(&row, 5, 8192)?.to_vec();
        Manifest::decode(&manifest)?;
        let value = RegistryRow {
            kind,
            artifact: rows::blob(&row, 1, 16)?
                .try_into()
                .map_err(|_| ManagedError::Corrupt)?,
            path: PathBuf::from(
                std::str::from_utf8(rows::blob(&row, 2, 4096)?)
                    .map_err(|_| ManagedError::Corrupt)?,
            ),
            bytes: rows::counter(&row, 3)?,
            digest: fixed32(rows::blob(&row, 4, 32)?)?,
            manifest,
        };
        if value.cells() != row || result.registry.insert(kind, value).is_some() {
            return Err(ManagedError::Corrupt);
        }
    }
    for row in rows::scan(c, rows::table("operation_receipts")?, 256)? {
        let value = ReceiptRow {
            sequence: rows::counter(&row, 0)?,
            operation: rows::blob(&row, 1, 16)?
                .try_into()
                .map_err(|_| ManagedError::Corrupt)?,
            result: rows::blob(&row, 2, 4096)?.to_vec(),
            digest: fixed32(rows::blob(&row, 3, 32)?)?,
        };
        if value.cells() != row || result.receipts.insert(value.sequence, value).is_some() {
            return Err(ManagedError::Corrupt);
        }
    }
    result.encode()?;
    Ok(result)
}
pub(super) fn load(
    c: &Connection,
    installation: [u8; 16],
    protected: CatalogFrontier,
) -> Result<Loaded> {
    super::schema::physical(c, Profile::Catalog)?;
    let meta = rows::scan(c, rows::table("catalog_meta")?, 1)?;
    let meta = meta.first().ok_or(ManagedError::Corrupt)?;
    let meta_revision = rows::counter(meta, 1)?;
    let meta_state = CatalogState::decode(rows::blob(meta, 2, 16_384)?)?;
    let meta_digest = fixed32(rows::blob(meta, 3, 32)?)?;
    let meta_head = fixed32(rows::blob(meta, 4, 32)?)?;
    if meta_revision == 0
        || meta_state.digest()? != meta_digest
        || meta_state.installation != installation
        || rows::blob(meta, 5, 32)? != super::schema::catalog_schema_digest()
        || meta_revision < protected.revision
    {
        return Err(ManagedError::StaleBackup);
    }
    let snapshots = rows::scan(c, rows::table("catalog_snapshots")?, 1)?;
    let snapshot = snapshots.first().ok_or(ManagedError::Corrupt)?;
    let checkpoint = rows::counter(snapshot, 0)?;
    let mut state = CatalogState::decode(rows::blob(snapshot, 1, 16_384)?)?;
    let mut digest = fixed32(rows::blob(snapshot, 2, 32)?)?;
    let mut head = fixed32(rows::blob(snapshot, 3, 32)?)?;
    let mut shadow = Maps::decode(rows::blob(snapshot, 4, 1_179_648)?)?;
    if checkpoint == 0
        || checkpoint > meta_revision
        || state.digest()? != digest
        || state.installation != installation
        || state.pending.is_some()
    {
        return Err(ManagedError::Corrupt);
    }
    catalog_rules::registry(&shadow, &state)?;
    let journal = rows::scan(c, rows::table("catalog_journal")?, 257)?;
    let mut guard = ReplayGuard::default();
    let mut revision = checkpoint;
    let mut anchor = protected.revision < checkpoint;
    if protected.revision == checkpoint {
        anchor = protected.state_digest == digest && protected.head == head;
    }
    if checkpoint == 1 {
        let genesis = journal.first().ok_or(ManagedError::Corrupt)?;
        if rows::counter(genesis, 0)? != 1
            || integer(genesis, 1)? != 1
            || rows::blob(genesis, 2, 32)? != [0; 32]
            || rows::blob(genesis, 3, 32)? != digest
            || rows::blob(genesis, 4, 32)? != head
            || rows::blob(genesis, 6, 16_384)? != state.encode()?
        {
            return Err(ManagedError::Corrupt);
        }
        let action = Action::decode(rows::blob(genesis, 5, 131_072)?)?;
        let zero = CatalogFrontier {
            revision: 0,
            state_digest: [0; 32],
            head: [0; 32],
        };
        if guard.advance(None, zero, &state, &Maps::default(), &action)? != shadow
            || catalog_state::journal_head(
                [0; 32],
                1,
                1,
                &action.encode()?,
                [0; 32],
                digest,
                state.registry,
            )? != head
        {
            return Err(ManagedError::Corrupt);
        }
    }
    let mut tail = 0;
    for row in journal {
        let r = rows::counter(&row, 0)?;
        if checkpoint == 1 && r == 1 {
            continue;
        }
        tail += 1;
        if tail > 256 || r <= checkpoint || revision.checked_add(1) != Some(r) {
            return Err(ManagedError::Corrupt);
        }
        let kind = integer(&row, 1)?;
        let action_bytes = rows::blob(&row, 5, 131_072)?;
        let action = Action::decode(action_bytes)?;
        let after = CatalogState::decode(rows::blob(&row, 6, 16_384)?)?;
        if action.payload.kind() != kind
            || rows::blob(&row, 2, 32)? != digest
            || rows::blob(&row, 3, 32)? != after.digest()?
        {
            return Err(ManagedError::Corrupt);
        }
        let frontier = CatalogFrontier {
            revision,
            state_digest: digest,
            head,
        };
        shadow = guard.advance(Some(&state), frontier, &after, &shadow, &action)?;
        head = catalog_state::journal_head(
            head,
            r,
            kind,
            action_bytes,
            digest,
            after.digest()?,
            after.registry,
        )?;
        if rows::blob(&row, 4, 32)? != head {
            return Err(ManagedError::Corrupt);
        }
        state = after;
        digest = state.digest()?;
        revision = r;
        if r == protected.revision {
            anchor = protected.state_digest == digest && protected.head == head;
        }
    }
    if !anchor
        || revision != meta_revision
        || state != meta_state
        || digest != meta_digest
        || head != meta_head
        || shadow != maps(c)?
    {
        return Err(ManagedError::Corrupt);
    }
    let pending_rows = rows::scan(c, rows::table("pending_world")?, 1)?;
    let pending = if let Some(expected) = state.pending {
        let row = pending_rows.first().ok_or(ManagedError::Corrupt)?;
        let bytes = rows::blob(row, 4, 5_242_880)?;
        let pending = Pending::decode(bytes)?;
        let summary = Summary::from_pending(&pending)?;
        let (recorded, utc) = guard.open.as_ref().ok_or(ManagedError::Corrupt)?;
        if hash(bytes) != expected
            || rows::blob(row, 5, 32)? != expected
            || rows::counter(row, 1)? != pending.sequence
            || rows::blob(row, 2, 16)? != pending.operation
            || integer(row, 3)? != pending.stage
            || summary.encode()? != recorded.encode()?
            || state.utc != *utc
            || pending.intended.utc != *utc
        {
            return Err(ManagedError::Corrupt);
        }
        Some(pending)
    } else {
        if !pending_rows.is_empty() || guard.open.is_some() {
            return Err(ManagedError::Corrupt);
        }
        None
    };
    Ok(Loaded {
        state,
        frontier: CatalogFrontier {
            revision,
            state_digest: digest,
            head,
        },
        maps: shadow,
        guard,
        checkpoint,
        pending,
    })
}
fn integer(row: &Row, at: usize) -> Result<u32> {
    match row.get(at) {
        Some(Cell::Integer(value)) => (*value).try_into().map_err(|_| ManagedError::Corrupt),
        _ => Err(ManagedError::Corrupt),
    }
}
