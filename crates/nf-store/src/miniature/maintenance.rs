use super::{
    MiniatureKnownFrontiers, MiniatureOutbox, MiniatureStore,
    economic::current_authority,
    error::{MiniatureStoreError, Result},
};
use crate::StoreError;
use nf_contract::identity::OperationId;
use rusqlite::DatabaseName;
use std::path::Path;
impl MiniatureStore {
    pub fn outbox(&self) -> Result<Vec<MiniatureOutbox>> {
        self.ensure()?;
        Ok(self.state.outbox.values().copied().collect())
    }
    /// Trusted local delivery owner metadata only; no strategic effect or network acknowledgement.
    pub fn ack_outbox(&mut self, operation: OperationId) -> Result<()> {
        self.ensure()?;
        if !self.runtime.claimed {
            return Err(MiniatureStoreError::Unclaimed);
        }
        let validate = |c: &rusqlite::Connection, old: &super::state::State| -> Result<()> {
            let member = crate::identity::load(c, old.scope())?.ok_or(StoreError::Corrupt)?;
            current_authority(old, &member).map(|_| ())
        };
        self.persist(
            3,
            |c, old| {
                validate(c, old)?;
                let mut next = old.clone();
                if next.outbox.remove(&operation).is_none() {
                    return Err(StoreError::InvalidTransition.into());
                }
                Ok((next, operation.as_bytes().to_vec()))
            },
            |c, old, _| validate(c, old),
            &mut |_| Ok(()),
        )
    }
    /// Copies public durable state, without creating authority or publishing new minima.
    pub fn backup_to(&self, path: impl AsRef<Path>) -> Result<MiniatureKnownFrontiers> {
        let known = self.known_frontiers()?;
        crate::schema::reserve(path.as_ref())?;
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
    pub fn page_count(&self) -> Result<u32> {
        self.ensure()?;
        Ok(self
            .connection
            .pragma_query_value(None, "page_count", |r| r.get(0))?)
    }
}
