//! Read-only persisted inspection of a quiescent rollback-journal profile.
use super::{error::Result, model::*, state::State};
use crate::{KnownFrontiers, StoreError};
use nf_contract::identity::AuthorityTerm;
use rusqlite::{Connection, DatabaseName, TransactionBehavior};
use std::path::Path;

/// An immutable recovered snapshot. It grants no authority to act or write.
///
/// The host must keep the local file and directory quiescent while opening.
/// WAL databases and journal sidecars are refused; recovery is never attempted.
pub struct MiniatureReader {
    connection: Connection,
    state: State,
    snapshot: MiniatureSnapshot,
}

impl MiniatureReader {
    pub fn open_existing(
        path: impl AsRef<Path>,
        known: MiniatureKnownFrontiers,
    ) -> Result<Self> {
        let mut connection = crate::schema::read_only_connection(path.as_ref())?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let journal: String =
            transaction.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        if journal != "delete" {
            return Err(StoreError::UnsupportedSchema.into());
        }
        super::schema::verify(&transaction)?;
        let (state, revision, _) = super::recovery::recover(&transaction, known)?;
        let snapshot = MiniatureSnapshot {
            world: state.world.clone(),
            authority: state.authority,
            pending: state.pending.clone(),
            known: MiniatureKnownFrontiers {
                storage: KnownFrontiers {
                    scope: state.scope(),
                    event_sequence: state.world.metadata().event_sequence,
                    store_revision: revision,
                    membership_revision: crate::identity::revision(&transaction, state.scope())?,
                },
                minimum_authority_term: state.authority.map_or(AuthorityTerm(0), |a| a.term),
            },
        };
        transaction.commit()?;
        Ok(Self {
            connection,
            state,
            snapshot,
        })
    }

    pub fn snapshot(&self) -> &MiniatureSnapshot {
        &self.snapshot
    }

    /// Bounded public persisted data; these bytes grant no authorization.
    pub fn snapshot_bytes(&self) -> Result<Vec<u8>> {
        super::codec::encode(&self.state)
    }

    /// SQLite's actual main-database mode, rather than a filesystem permission guess.
    pub fn is_read_only(&self) -> Result<bool> {
        Ok(self.connection.is_readonly(DatabaseName::Main)?)
    }
}
