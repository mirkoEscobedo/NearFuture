use crate::{
    codec,
    error::{Result, StoreError},
    model::*,
    schema::{self, counter, hash},
};
use nf_contract::identity::*;
use nf_kernel::{CommittedBatch, Frontier, Intent, World};
use rusqlite::{Connection, DatabaseName, TransactionBehavior, params};
use std::path::Path;
/// Owns the only writer connection. The type has no shared concurrent access or network transport.
pub struct Store {
    pub(crate) connection: Connection,
    pub(crate) state: State,
    pub(crate) quarantined: bool,
    revision: u64,
    head: [u8; 32],
}
#[derive(Clone, Debug)]
pub struct ImmutableSnapshot {
    revision: u64,
    bytes: Vec<u8>,
    digest: [u8; 32],
}
impl ImmutableSnapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
}
impl Store {
    pub fn create(path: impl AsRef<Path>, world: &World) -> Result<Self> {
        if world.view().event_seq() != EventSeq(0) || !world.outcomes().is_empty() {
            return Err(StoreError::InvalidTransition);
        }
        let state = State::initial(world);
        codec::encode(&state)?;
        schema::reserve(path.as_ref())?;
        let mut connection = schema::connection(path.as_ref())?;
        schema::configure(&connection)?;
        crate::persistence::initialize(&mut connection, &state)?;
        Ok(Self {
            connection,
            state,
            quarantined: false,
            revision: 0,
            head: [0; 32],
        })
    }
    pub fn open_existing(path: impl AsRef<Path>, known: KnownFrontiers) -> Result<Self> {
        let metadata = std::fs::metadata(path.as_ref()).map_err(|_| StoreError::MissingHistory)?;
        if metadata.len() > 268_435_456 {
            return Err(StoreError::Limit);
        }
        if metadata.len() < 100 {
            return Err(StoreError::Corrupt);
        }
        let connection = schema::connection(path.as_ref())?;
        schema::verify_existing(&connection)?;
        schema::configure(&connection)?;
        let (state, revision, head) = crate::recovery::recover(&connection, known)?;
        Ok(Self {
            connection,
            state,
            revision,
            head,
            quarantined: false,
        })
    }
    pub(crate) fn ensure_writable(&self) -> Result<()> {
        if self.quarantined {
            Err(StoreError::Quarantined)
        } else {
            Ok(())
        }
    }
    pub fn world(&self) -> &World {
        &self.state.world
    }
    pub fn pending(&self) -> Option<&Frontier> {
        self.state.pending.as_ref()
    }
    pub fn reservations(&self) -> impl Iterator<Item = &Reservation> {
        self.state.reservations.values()
    }
    pub fn outbox(&self) -> impl Iterator<Item = &OutboxRecord> {
        self.state.outbox.values()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn known_frontiers(&self) -> Result<KnownFrontiers> {
        Ok(KnownFrontiers {
            scope: self.state.scope(),
            event_sequence: self.state.world.view().event_seq(),
            store_revision: self.revision,
            membership_revision: crate::identity::revision(&self.connection, self.state.scope())?,
        })
    }
    pub fn query(&self, intent: &Intent, device: DeviceId) -> Result<Option<RequestStatus>> {
        self.ensure_writable()?;
        crate::transitions::query(&self.state, intent, device)
    }
    /// Data-only lookup. The caller must authenticate and authorize these identities first.
    pub fn query_bound(
        &self,
        request: RequestId,
        account: AccountId,
        device: DeviceId,
        scope: nf_identity::model::Scope,
    ) -> Result<Option<BoundRequestStatus>> {
        self.ensure_writable()?;
        if scope != self.state.scope() {
            return Err(StoreError::Scope);
        }
        let Some(record) = self.state.requests.get(&request) else {
            return Ok(None);
        };
        if account != record.intent.actor || device != record.device {
            return Err(StoreError::RequestConflict);
        }
        Ok(Some(BoundRequestStatus {
            status: record.status,
            binding_digest: binding(&record.intent, record.device)?.digest(),
        }))
    }
    pub fn prepare(
        &mut self,
        frontier: &Frontier,
        devices: &[PrincipalDevice],
        reservations: &[Reservation],
    ) -> Result<Accepted> {
        self.ensure_writable()?;
        let next = crate::transitions::prepare(&self.state, frontier, devices, reservations)?;
        let action = nf_kernel::encode_frontier(frontier)?;
        self.persist(next, 1, &action, &mut |_| Ok(()))?;
        Ok(Accepted::new(self.revision))
    }
    pub fn commit(&mut self, batch: &CommittedBatch) -> Result<DurableAck> {
        self.commit_with_hook(batch, &mut |_| Ok(()))
    }
    /// Optional bounded fault/observation seam. A post-commit failure requires reopen/query to resolve uncertainty.
    pub fn commit_with_hook(
        &mut self,
        batch: &CommittedBatch,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<DurableAck> {
        self.ensure_writable()?;
        let next = crate::transitions::commit(&self.state, batch)?;
        let action = nf_kernel::encode_batch(batch)?;
        self.persist(next, 2, &action, hook)?;
        Ok(DurableAck {
            revision: self.revision,
            sequence: batch.sequence,
            state_hash: batch.after_hash,
        })
    }
    fn persist(
        &mut self,
        next: State,
        kind: u32,
        action: &[u8],
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<()> {
        let revision = self.revision.checked_add(1).ok_or(StoreError::Limit)?;
        codec::encode(&next)?;
        hook(Boundary::BeforeTransaction)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let head = crate::persistence::write(
            &transaction,
            &self.state,
            &next,
            revision,
            self.head,
            kind,
            action,
        )?;
        hook(Boundary::AfterWrites)?;
        if revision % 128 == 0 {
            checkpoint(&transaction, &next, revision, head)?;
        }
        hook(Boundary::BeforeCommit)?;
        if let Err(error) = transaction.commit() {
            self.quarantined = true;
            return Err(error.into());
        }
        if hook(Boundary::AfterCommit).is_err() || hook(Boundary::BeforeAcknowledgement).is_err() {
            self.quarantined = true;
            return Err(StoreError::UncertainCommit);
        }
        self.state = next;
        self.revision = revision;
        self.head = head;
        Ok(())
    }
    pub fn mark_outbox_delivered(&mut self, operation: OperationId) -> Result<()> {
        self.ensure_writable()?;
        let mut next = self.state.clone();
        if next.outbox.remove(&operation).is_none() {
            return Err(StoreError::InvalidTransition);
        }
        self.persist(next, 3, operation.as_bytes(), &mut |_| Ok(()))
    }
    pub fn snapshot(&self) -> Result<ImmutableSnapshot> {
        self.ensure_writable()?;
        let bytes = codec::encode(&self.state)?;
        Ok(ImmutableSnapshot {
            revision: self.revision,
            digest: hash(&bytes),
            bytes,
        })
    }
    pub fn compact(&mut self) -> Result<()> {
        self.ensure_writable()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        checkpoint(&transaction, &self.state, self.revision, self.head)?;
        if let Err(error) = transaction.commit() {
            self.quarantined = true;
            return Err(error.into());
        }
        Ok(())
    }
    pub fn backup_to(&self, path: impl AsRef<Path>) -> Result<KnownFrontiers> {
        self.ensure_writable()?;
        let known = self.known_frontiers()?;
        schema::reserve(path.as_ref())?;
        self.connection
            .backup(DatabaseName::Main, path.as_ref(), None)?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())
            .map_err(|_| StoreError::Io)?
            .sync_all()
            .map_err(|_| StoreError::Io)?;
        Ok(known)
    }
    /// Lower the database growth quota. Used for deployment limits and genuine SQLITE_FULL testing.
    pub fn lower_page_limit(&mut self, pages: u32) -> Result<()> {
        self.ensure_writable()?;
        let previous: u32 = self
            .connection
            .pragma_query_value(None, "max_page_count", |row| row.get(0))?;
        let current: u32 = self
            .connection
            .pragma_query_value(None, "page_count", |row| row.get(0))?;
        if pages < current || pages > previous {
            return Err(StoreError::Limit);
        }
        self.connection
            .pragma_update(None, "max_page_count", pages)?;
        let actual: u32 = self
            .connection
            .pragma_query_value(None, "max_page_count", |row| row.get(0))?;
        if actual != pages {
            return Err(StoreError::Io);
        }
        Ok(())
    }
    pub fn page_count(&self) -> Result<u32> {
        Ok(self
            .connection
            .pragma_query_value(None, "page_count", |row| row.get(0))?)
    }
}
fn checkpoint(connection: &Connection, state: &State, revision: u64, head: [u8; 32]) -> Result<()> {
    let bytes = codec::encode(state)?;
    let digest = hash(&bytes);
    connection.execute(
        "INSERT INTO snapshots VALUES (?1,?2,?3,?4) ON CONFLICT(revision) DO NOTHING",
        params![counter(revision), bytes, digest, head],
    )?;
    let stored: (Vec<u8>, Vec<u8>) = connection.query_row(
        "SELECT digest,head FROM snapshots WHERE revision=?1",
        params![counter(revision)],
        |row| {
            Ok((
                crate::persistence::bounded_blob(row, 0, 32)?,
                crate::persistence::bounded_blob(row, 1, 32)?,
            ))
        },
    )?;
    if stored.0 != digest || stored.1 != head {
        return Err(StoreError::Corrupt);
    }
    connection.execute(
        "DELETE FROM snapshots WHERE revision<?1",
        params![counter(revision)],
    )?;
    connection.execute(
        "DELETE FROM journal WHERE revision<=?1",
        params![counter(revision)],
    )?;
    connection.execute(
        "DELETE FROM events WHERE sequence<=?1",
        params![counter(state.world.view().event_seq().0)],
    )?;
    Ok(())
}
