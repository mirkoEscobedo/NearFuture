use super::{MiniatureStore, codec, error::Result, state::State};
use crate::{
    Boundary, StoreError,
    persistence::bounded_blob,
    schema::{counter, hash, read_counter},
};
use nf_kernel::miniature::*;
use rusqlite::{Connection, TransactionBehavior, params};
impl MiniatureStore {
    pub(crate) fn persist(
        &mut self,
        kind: u32,
        build: impl FnOnce(&Connection, &State) -> Result<(State, Vec<u8>)>,
        final_cut: impl FnOnce(&Connection, &State, &State) -> Result<()>,
        hook: &mut impl FnMut(Boundary) -> Result<()>,
    ) -> Result<()> {
        self.ensure()?;
        let revision = self.revision.checked_add(1).ok_or(StoreError::Limit)?;
        hook(Boundary::BeforeTransaction)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_current(&tx, &self.state, self.revision, self.head)?;
        let (next, action) = build(&tx, &self.state)?;
        if action.len() > 1_048_576 {
            return Err(StoreError::Limit.into());
        }
        let head = match write(&tx, &self.state, &next, revision, self.head, kind, &action) {
            Ok(head) => head,
            Err(error) => {
                quarantine_storage_failure(&mut self.quarantined, &mut self.runtime, error);
                return Err(error);
            }
        };
        hook(Boundary::AfterWrites)?;
        if revision % 128 == 0
            && let Err(error) = checkpoint(&tx, &next, revision, head)
        {
            quarantine_storage_failure(&mut self.quarantined, &mut self.runtime, error);
            return Err(error);
        }
        hook(Boundary::BeforeCommit)?;
        // The admitted policy and production monotonic deadlines are checked after all hooks/writes.
        verify_current(&tx, &next, revision, head)?;
        final_cut(&tx, &self.state, &next)?;
        if let Err(error) = tx.commit() {
            self.quarantined = true;
            self.runtime.clear();
            self.runtime.claimed = false;
            return Err(error.into());
        }
        if hook(Boundary::AfterCommit).is_err() || hook(Boundary::BeforeAcknowledgement).is_err() {
            self.quarantined = true;
            self.runtime.clear();
            self.runtime.claimed = false;
            return Err(StoreError::UncertainCommit.into());
        }
        self.state = next;
        self.revision = revision;
        self.head = head;
        Ok(())
    }
}
pub(crate) fn verify_current(
    c: &Connection,
    old: &State,
    revision: u64,
    head: [u8; 32],
) -> Result<()> {
    let (rev, bytes, digest, actual_head) = c.query_row(
        "SELECT revision,state,digest,head FROM history_meta WHERE singleton=1",
        [],
        |r| {
            Ok((
                bounded_blob(r, 0, 8)?,
                bounded_blob(r, 1, 1_048_576)?,
                bounded_blob(r, 2, 32)?,
                bounded_blob(r, 3, 32)?,
            ))
        },
    )?;
    if read_counter(&rev)? != revision
        || actual_head != head
        || hash(&bytes) != digest.as_slice()
        || bytes != codec::encode(old)?
    {
        return Err(StoreError::Corrupt.into());
    }
    Ok(())
}
fn write(
    c: &Connection,
    old: &State,
    next: &State,
    revision: u64,
    previous_head: [u8; 32],
    kind: u32,
    action: &[u8],
) -> Result<[u8; 32]> {
    let bytes = codec::encode(next)?;
    let digest = hash(&bytes);
    let previous = hash(&codec::encode(old)?);
    let head = crate::persistence::head(previous_head, revision, kind, action, digest);
    c.execute(
        "INSERT INTO journal VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            counter(revision),
            kind,
            previous,
            digest,
            head,
            action,
            bytes
        ],
    )?;
    if kind == 2 {
        let batch = decode_miniature_batch(action, &old.world)?;
        c.execute(
            "INSERT INTO events VALUES (?1,?2,?3)",
            params![counter(batch.event_sequence().0), action, hash(action)],
        )?;
    }
    c.execute("UPDATE history_meta SET revision=?1,event_seq=?2,state=?3,digest=?4,head=?5 WHERE singleton=1",params![counter(revision),counter(next.world.metadata().event_sequence.0),bytes,digest,head])?;
    super::mirrors::write(c, next)?;
    Ok(head)
}
pub(crate) fn checkpoint(
    c: &Connection,
    state: &State,
    revision: u64,
    head: [u8; 32],
) -> Result<()> {
    let bytes = codec::encode(state)?;
    c.execute(
        "INSERT OR REPLACE INTO snapshots VALUES (?1,?2,?3,?4)",
        params![counter(revision), bytes, hash(&bytes), head],
    )?;
    c.execute(
        "DELETE FROM snapshots WHERE revision<?1",
        params![counter(revision)],
    )?;
    c.execute(
        "DELETE FROM journal WHERE revision<=?1",
        params![counter(revision)],
    )?;
    c.execute(
        "DELETE FROM events WHERE sequence<=?1",
        params![counter(state.world.metadata().event_sequence.0)],
    )?;
    Ok(())
}

fn quarantine_storage_failure(
    quarantined: &mut bool,
    runtime: &mut super::auth::AuthRuntime,
    error: super::MiniatureStoreError,
) {
    if matches!(
        error,
        super::MiniatureStoreError::Storage(
            StoreError::Full | StoreError::Io | StoreError::Corrupt
        )
    ) {
        *quarantined = true;
        runtime.clear();
        runtime.claimed = false;
    }
}
