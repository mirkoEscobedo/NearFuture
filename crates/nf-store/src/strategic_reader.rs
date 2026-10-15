//! Read-only inspection of one admitted, quiescent Profile1 history.
mod observation;
use crate::{ImmutableSnapshot, KnownFrontiers, error::Result, model::State};
pub use observation::{
    StrategicSupportAssignmentV1, StrategicSupportCountsV1, StrategicSupportImplementation,
    StrategicSupportManifestV1, StrategicSupportObservationV1, StrategicSupportProtectionV1,
    StrategicSupportRulesetBinding, StrategicSupportUnavailable, SupportProtection,
};
use rusqlite::{Connection, DatabaseName, TransactionBehavior};
use std::{fmt, path::Path};

/// A recovered data view, with no writer or authority conversion.
///
/// The caller must own and keep the local file and ancestors quiescent.
/// Protected minima must be retained independently of this file. Journal sidecars
/// and WAL profiles are refused; no recovery, retry or filesystem cleanup occurs.
/// Raw snapshots are private-data inputs, never safe support-export bytes.
pub struct StrategicReader {
    connection: Connection,
    state: State,
    known: KnownFrontiers,
    observation: StrategicSupportObservationV1,
}

impl StrategicReader {
    pub fn open_existing(path: impl AsRef<Path>, known: KnownFrontiers) -> Result<Self> {
        let mut connection =
            crate::strategic_reader_connection::read_only_connection(path.as_ref())?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let journal: String =
            transaction.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        if journal != "delete" {
            return Err(crate::StoreError::UnsupportedSchema);
        }
        crate::schema::verify_existing(&transaction)?;
        let (state, revision, _) = crate::recovery::recover(&transaction, known)?;
        let membership = crate::identity::load(&transaction, state.scope())?;
        let actual = KnownFrontiers {
            scope: state.scope(),
            event_sequence: state.world.view().event_seq(),
            store_revision: revision,
            membership_revision: membership.as_ref().map(|state| state.revision),
        };
        let accounts = membership
            .as_ref()
            .map(|state| observation::count(state.accounts.len()))
            .transpose()?;
        let observation =
            StrategicSupportObservationV1::from_recovered(&state, actual, known, accounts)?;
        transaction.commit()?;
        Ok(Self {
            connection,
            state,
            known: actual,
            observation,
        })
    }

    /// Reads SQLite's actual mode and the required connection-local protections.
    pub fn is_read_only(&self) -> Result<bool> {
        let query_only: i32 = self
            .connection
            .pragma_query_value(None, "query_only", |row| row.get(0))?;
        let trusted: i32 = self
            .connection
            .pragma_query_value(None, "trusted_schema", |row| row.get(0))?;
        Ok(self.connection.is_readonly(DatabaseName::Main)? && query_only == 1 && trusted == 0)
    }

    pub fn known_frontiers(&self) -> Result<KnownFrontiers> {
        Ok(self.known)
    }

    pub fn snapshot(&self) -> Result<ImmutableSnapshot> {
        ImmutableSnapshot::from_recovered(&self.state, self.known.store_revision)
    }

    pub fn support_observation(&self) -> Result<StrategicSupportObservationV1> {
        Ok(self.observation.clone())
    }
}

impl fmt::Debug for StrategicReader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StrategicReader { read_only: true, evidence_is_not_authority: true }")
    }
}
