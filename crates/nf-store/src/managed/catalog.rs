use super::{bytes::{hash, next}, catalog_action::Action, catalog_payload::Payload,
    catalog_read, catalog_reduce::ReplayGuard, catalog_state::{self, CatalogState},
    error::{ManagedError, Result}, maps::Maps, pending::Pending,
    rows::{self, Patch}, schema::{self, Profile}, stamp::CatalogFrontier};
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use std::path::PathBuf;

pub(super) enum PendingChange { Keep, Set(Pending), Clear }
pub(super) enum CreateFailure { Before(ManagedError), After(ManagedError) }
pub(super) struct Catalog {
    pub connection: Connection,
    pub path: PathBuf,
    pub state: CatalogState,
    pub frontier: CatalogFrontier,
    pub maps: Maps,
    pub pending: Option<Pending>,
    guard: ReplayGuard,
    checkpoint: u64,
    pub quarantined: bool,
}
impl Catalog {
    pub fn open(path: PathBuf, installation: [u8; 16], protected: CatalogFrontier) -> Result<Self> {
        schema::local_absolute(&path)?; schema::companions_absent(&path, true)?;
        let meta = std::fs::symlink_metadata(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound { ManagedError::MissingCatalog } else { e.into() }
        })?;
        schema::regular_file(&meta)?;
        if meta.len() > 67_108_864 {
            return Err(ManagedError::UnsupportedFormat);
        }
        schema::header(&path, Profile::Catalog)?;
        let c = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        schema::limits(&c, Profile::Catalog)?;
        schema::physical(&c, Profile::Catalog)?; schema::configure(&c, Profile::Catalog, 16_384)?;
        let loaded = catalog_read::load(&c, installation, protected)?;
        Ok(Self { connection: c, path, state: loaded.state, frontier: loaded.frontier, maps: loaded.maps,
            guard: loaded.guard, pending: loaded.pending, checkpoint: loaded.checkpoint, quarantined: false })
    }
    pub fn create(path: PathBuf, state: CatalogState, action: Action) -> std::result::Result<Self, CreateFailure> {
        let before = |error| CreateFailure::Before(error);
        let reservation = schema::reserve(&path).map_err(before)?; drop(reservation);
        let mut c = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .map_err(|e| before(e.into()))?;
        schema::limits(&c, Profile::Catalog).map_err(before)?;
        c.pragma_update(None, "page_size", 4096).map_err(|e| before(e.into()))?;
        schema::configure(&c, Profile::Catalog, 16_384).map_err(before)?;
        let zero = CatalogFrontier { revision: 0, state_digest: [0; 32], head: [0; 32] };
        let mut guard = ReplayGuard::default();
        let maps = guard.advance(None, zero, &state, &Maps::default(), &action).map_err(before)?;
        let bytes = state.encode().map_err(before)?; let digest = hash(&bytes); let action_bytes = action.encode().map_err(before)?;
        let head = catalog_state::journal_head([0; 32], 1, 1, &action_bytes, [0; 32], digest, state.registry).map_err(before)?;
        let tx = c.transaction_with_behavior(TransactionBehavior::Exclusive).map_err(|e| before(e.into()))?;
        tx.execute_batch(schema::CATALOG_SCHEMA).map_err(|e| before(e.into()))?;
        tx.pragma_update(None, "application_id", Profile::Catalog.id()).map_err(|e| before(e.into()))?;
        tx.pragma_update(None, "user_version", 1).map_err(|e| before(e.into()))?;
        apply_maps(&tx, &action).map_err(before)?;
        tx.execute("INSERT INTO catalog_meta VALUES(1,?1,?2,?3,?4,?5)",
            params![1_u64.to_be_bytes(), bytes, digest, head, schema::catalog_schema_digest()]).map_err(|e| before(e.into()))?;
        tx.execute("INSERT INTO catalog_journal VALUES(?1,1,?2,?3,?4,?5,?6)",
            params![1_u64.to_be_bytes(), [0_u8; 32], digest, head, action_bytes, bytes]).map_err(|e| before(e.into()))?;
        tx.execute("INSERT INTO catalog_snapshots VALUES(?1,?2,?3,?4,?5)",
            params![1_u64.to_be_bytes(), bytes, digest, head, maps.encode().map_err(before)?]).map_err(|e| before(e.into()))?;
        tx.commit().map_err(|_| CreateFailure::After(ManagedError::UncertainOutcome))?;
        schema::close_sync(c, &path).map_err(CreateFailure::After)?;
        Self::open(path, state.installation, CatalogFrontier { revision: 1, state_digest: digest, head }).map_err(CreateFailure::After)
    }
    pub fn verify_cached(&self) -> Result<()> {
        if self.quarantined { return Err(ManagedError::Quarantined); }
        let actual = catalog_read::load(&self.connection, self.state.installation, self.frontier)?;
        if actual.state != self.state || actual.frontier != self.frontier || actual.maps != self.maps {
            return Err(ManagedError::Corrupt);
        }
        Ok(())
    }
    pub fn append(&mut self, action: Action, after: CatalogState, pending: PendingChange) -> Result<()> {
        self.append_step(action, after, pending, false)?;
        if self.state.pending.is_none() && self.frontier.revision.checked_sub(self.checkpoint).ok_or(ManagedError::Corrupt)? >= 128 {
            let action = Action { payload: Payload::Checkpoint { revision: self.frontier.revision,
                state: self.frontier.state_digest, head: self.frontier.head, rows: hash(&self.maps.encode()?) },
                delta: super::delta::Delta::default() };
            self.append_step(action, self.state.clone(), PendingChange::Keep, true)?;
        }
        Ok(())
    }
    fn append_step(&mut self, action: Action, after: CatalogState, change: PendingChange, checkpoint: bool) -> Result<()> {
        self.verify_cached()?;
        let revision = next(self.frontier.revision)?; let mut guard = self.guard.clone();
        let maps = guard.advance(Some(&self.state), self.frontier, &after, &self.maps, &action)?;
        let pending = match change {
            PendingChange::Keep => self.pending.clone(), PendingChange::Set(value) => Some(value), PendingChange::Clear => None,
        };
        if after.pending != pending.as_ref().map(Pending::digest).transpose()?
            || pending.as_ref().is_some_and(|p| p.sequence != after.consumed)
        { return Err(ManagedError::Corrupt); }
        let bytes = after.encode()?; let digest = hash(&bytes); let action_bytes = action.encode()?;
        let head = catalog_state::journal_head(self.frontier.head, revision, action.payload.kind(),
            &action_bytes, self.frontier.state_digest, digest, after.registry)?;
        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Exclusive)?;
        apply_maps(&tx, &action)?;
        tx.execute("DELETE FROM pending_world", [])?;
        if let Some(pending) = &pending {
            let record = pending.encode()?;
            tx.execute("INSERT INTO pending_world VALUES(1,?1,?2,?3,?4,?5)",
                params![pending.sequence.to_be_bytes(), pending.operation, pending.stage, record, hash(&record)])?;
        }
        tx.execute("INSERT INTO catalog_journal VALUES(?1,?2,?3,?4,?5,?6,?7)", params![revision.to_be_bytes(),
            action.payload.kind(), self.frontier.state_digest, digest, head, action_bytes, bytes])?;
        if tx.execute("UPDATE catalog_meta SET revision=?1,state=?2,digest=?3,head=?4 WHERE singleton=1",
            params![revision.to_be_bytes(), bytes, digest, head])? != 1 { return Err(ManagedError::Corrupt); }
        if checkpoint {
            if pending.is_some() { return Err(ManagedError::OperationConflict); }
            tx.execute("INSERT INTO catalog_snapshots VALUES(?1,?2,?3,?4,?5)",
                params![revision.to_be_bytes(), bytes, digest, head, maps.encode()?])?;
            tx.execute("DELETE FROM catalog_snapshots WHERE revision<>?1", params![revision.to_be_bytes()])?;
            tx.execute("DELETE FROM catalog_journal WHERE revision<=?1", params![revision.to_be_bytes()])?;
        }
        if tx.commit().is_err() { self.quarantined = true; return Err(ManagedError::UncertainOutcome); }
        self.state = after; self.frontier = CatalogFrontier { revision, state_digest: digest, head };
        self.maps = maps; self.guard = guard; self.pending = pending;
        if checkpoint { self.checkpoint = revision; }
        Ok(())
    }
    pub fn close(self) -> Result<CatalogFrontier> {
        if self.quarantined || self.pending.is_some() { return Err(ManagedError::ReconciliationRequired); }
        let frontier = self.frontier; schema::close_sync(self.connection, &self.path)?; Ok(frontier)
    }
}
fn apply_maps(c: &Connection, action: &Action) -> Result<()> {
    let registry = rows::table("artifact_registry")?; let receipts = rows::table("operation_receipts")?;
    let mut patch = Patch::default();
    for (before, after) in action.delta.registry.values() {
        patch.put(registry, before.as_ref().map(|r| r.cells()), after.as_ref().map(|r| r.cells()))?;
    }
    patch.apply(c, registry)?;
    let mut patch = Patch::default();
    for (before, after) in action.delta.receipts.values() {
        patch.put(receipts, before.as_ref().map(|r| r.cells()), after.as_ref().map(|r| r.cells()))?;
    }
    patch.apply(c, receipts)
}
