use super::{
    active_plan, active_read,
    error::{ManagedError, Result},
    foot::StorageFoot,
    mutation::Mutation,
    retired_read,
    schema::{self, Profile},
    source_plan,
    state::FullState,
    world_plan::Plan,
};
use crate::{KnownFrontiers, Store};
use nf_kernel::CommittedBatch;
use rusqlite::{Connection, TransactionBehavior};
use std::path::{Path, PathBuf};

pub(super) struct WorldFile {
    pub connection: Connection,
    pub path: PathBuf,
    pub state: FullState,
    pub foot: StorageFoot,
    pub core_bytes: Vec<u8>,
    pub envelope_bytes: Vec<u8>,
    pub quarantined: bool,
}
pub(super) enum Attempt {
    KnownPost,
    KnownPre(ManagedError),
    Unknown(ManagedError),
}
impl WorldFile {
    pub fn consume_source(
        source: Store,
        path: PathBuf,
        minimum: KnownFrontiers,
    ) -> std::result::Result<Self, (ManagedError, Store)> {
        let checked = (|| {
            super::journal::cold(&source.connection, &path)?;
            source.verified_source_head(minimum)?;
            let quota: u32 = source
                .connection
                .pragma_query_value(None, "max_page_count", |r| r.get(0))?;
            let member = crate::identity::load(&source.connection, minimum.scope)?
                .ok_or(ManagedError::Unauthorized)?;
            let (state, foot) = source_plan::inspect(&source, member, quota, None, None)?;
            if state.core.pending.is_some() {
                return Err(ManagedError::NotDrained);
            }
            quota_check(&source.connection, quota)?;
            let core_bytes = crate::codec::encode(&state.core)?;
            let envelope_bytes = state.encode()?;
            Ok((state, foot, core_bytes, envelope_bytes))
        })();
        match checked {
            Ok((state, foot, core_bytes, envelope_bytes)) => Ok(Self {
                connection: source.connection,
                path,
                state,
                foot,
                core_bytes,
                envelope_bytes,
                quarantined: false,
            }),
            Err(error) => Err((error, source)),
        }
    }
    pub fn open(
        path: PathBuf,
        expected: &StorageFoot,
        owned_pending: bool,
        retirement: Option<[u8; 32]>,
    ) -> Result<Self> {
        let admission = if owned_pending {
            super::journal::Admission::OwnedRecovery
        } else {
            super::journal::Admission::Absent
        };
        Self::open_admitted(path, expected, admission, retirement)
    }
    /// Only actual catalog-bound reopen callers select the cold-journal exception.
    pub fn open_owned(
        path: PathBuf,
        expected: &StorageFoot,
        retirement: Option<[u8; 32]>,
    ) -> Result<Self> {
        Self::open_admitted(
            path,
            expected,
            super::journal::Admission::OwnedCold,
            retirement,
        )
    }
    fn open_admitted(
        path: PathBuf,
        expected: &StorageFoot,
        admission: super::journal::Admission,
        retirement: Option<[u8; 32]>,
    ) -> Result<Self> {
        schema::local_absolute(&path)?;
        super::journal::admit(&path, admission)?;
        let meta = std::fs::symlink_metadata(&path)?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 268_435_456 {
            return Err(ManagedError::UnsupportedFormat);
        }
        let profile = profile(expected.profile)?;
        let connection = schema::open(&path, profile, expected.quota, admission)?;
        super::journal::cold(&connection, &path)?;
        let state = inspect(&connection, expected, retirement)?;
        let core_bytes = crate::codec::encode(&state.core)?;
        let envelope_bytes = state.encode()?;
        Ok(Self {
            connection,
            path,
            state,
            foot: expected.clone(),
            core_bytes,
            envelope_bytes,
            quarantined: false,
        })
    }
    pub fn verify(&self, expected: &StorageFoot, retirement: Option<[u8; 32]>) -> Result<()> {
        if self.quarantined {
            return Err(ManagedError::Quarantined);
        }
        if &self.foot != expected {
            return Err(ManagedError::SourceMismatch);
        }
        super::journal::cold(&self.connection, &self.path)?;
        quota_check(&self.connection, expected.quota)?;
        let state = inspect(&self.connection, expected, retirement)?;
        if state != self.state
            || crate::codec::encode(&state.core)? != self.core_bytes
            || state.encode()? != self.envelope_bytes
        {
            return Err(ManagedError::SourceMismatch);
        }
        Ok(())
    }
    pub fn plan(
        &self,
        state: FullState,
        mutation: &Mutation,
        action: &[u8],
        batch: Option<&CommittedBatch>,
    ) -> Result<Plan> {
        match self.foot.profile {
            1 => source_plan::plan(
                &self.connection,
                &self.state,
                &self.foot,
                state,
                mutation,
                batch,
            ),
            4 => active_plan::plan(
                &self.connection,
                &self.foot,
                state,
                mutation.purpose(),
                action,
                batch,
            ),
            _ => Err(ManagedError::UnsupportedFormat),
        }
    }
    pub fn apply(&mut self, plan: &Plan, post: &StorageFoot) -> Attempt {
        let before = self.foot.clone();
        if before.profile != post.profile || before.profile == 5 {
            return Attempt::KnownPre(ManagedError::UnsupportedFormat);
        }
        let lowered = post.quota < before.quota;
        if lowered {
            if let Err(error) = self
                .connection
                .pragma_update(None, "max_page_count", post.quota)
                .map_err(ManagedError::from)
                .and_then(|_| quota_check(&self.connection, post.quota))
            {
                return self.restore_pre(&before, error);
            }
        }
        let tx = match self
            .connection
            .transaction_with_behavior(TransactionBehavior::Exclusive)
        {
            Ok(tx) => tx,
            Err(error) => {
                self.quarantined = true;
                return Attempt::Unknown(error.into());
            }
        };
        let writes = plan
            .apply(&tx)
            .and_then(|_| quota_check(&tx, post.quota))
            .and_then(|_| inspect(&tx, post, None))
            .and_then(|state| {
                if state == plan.state {
                    Ok(())
                } else {
                    Err(ManagedError::DerivationMismatch)
                }
            });
        if let Err(error) = writes {
            if tx.rollback().is_err() {
                self.quarantined = true;
                return Attempt::Unknown(ManagedError::UncertainOutcome);
            }
            return self.restore_pre(&before, error);
        }
        if tx.commit().is_err() {
            self.quarantined = true;
            return Attempt::Unknown(ManagedError::UncertainOutcome);
        }
        let checked = inspect(&self.connection, post, None).and_then(|state| {
            if state != plan.state {
                Err(ManagedError::DerivationMismatch)
            } else {
                Ok(state)
            }
        });
        match checked {
            Ok(state) => {
                let core = crate::codec::encode(&state.core).map_err(ManagedError::from);
                let envelope = state.encode();
                match (core, envelope) {
                    (Ok(core), Ok(envelope)) => {
                        self.state = state;
                        self.foot = post.clone();
                        self.core_bytes = core;
                        self.envelope_bytes = envelope;
                        Attempt::KnownPost
                    }
                    _ => {
                        self.quarantined = true;
                        Attempt::Unknown(ManagedError::ReconciliationRequired)
                    }
                }
            }
            Err(error) => {
                self.quarantined = true;
                Attempt::Unknown(error)
            }
        }
    }
    pub fn apply_source_quota(&mut self, plan: &Plan, post: &StorageFoot) -> Attempt {
        let before = self.foot.clone();
        if before.profile != 1
            || post.profile != 1
            || !plan.patches.is_empty()
            || post.quota >= before.quota
        {
            return Attempt::KnownPre(ManagedError::OperationConflict);
        }
        let set = self
            .connection
            .pragma_update(None, "max_page_count", post.quota)
            .map_err(ManagedError::from)
            .and_then(|_| quota_check(&self.connection, post.quota))
            .and_then(|_| inspect(&self.connection, post, None));
        match set {
            Ok(state) if state == plan.state => {
                match (crate::codec::encode(&state.core), state.encode()) {
                    (Ok(core), Ok(envelope)) => {
                        self.state = state;
                        self.foot = post.clone();
                        self.core_bytes = core;
                        self.envelope_bytes = envelope;
                        Attempt::KnownPost
                    }
                    _ => {
                        self.quarantined = true;
                        Attempt::Unknown(ManagedError::ReconciliationRequired)
                    }
                }
            }
            Ok(_) => self.restore_pre(&before, ManagedError::DerivationMismatch),
            Err(error) => self.restore_pre(&before, error),
        }
    }
    fn restore_pre(&mut self, before: &StorageFoot, error: ManagedError) -> Attempt {
        let restored = self
            .connection
            .pragma_update(None, "max_page_count", before.quota)
            .map_err(ManagedError::from)
            .and_then(|_| quota_check(&self.connection, before.quota))
            .and_then(|_| inspect(&self.connection, before, None));
        if restored.is_err() {
            self.quarantined = true;
            Attempt::Unknown(ManagedError::ReconciliationRequired)
        } else {
            Attempt::KnownPre(error)
        }
    }
    pub fn close(self) -> Result<()> {
        schema::close_sync(self.connection, &self.path)
    }
}
pub(super) fn inspect(
    c: &Connection,
    expected: &StorageFoot,
    retirement: Option<[u8; 32]>,
) -> Result<FullState> {
    quota_check(c, expected.quota)?;
    match expected.profile {
        1 => Ok(source_plan::inspect_connection(
            c,
            expected.known,
            expected.quota,
            expected.policy,
            Some(expected),
        )?
        .0),
        4 => Ok(active_read::inspect(c, expected.known, Some(expected))?.state),
        5 => Ok(retired_read::inspect(c, expected, retirement)?.0),
        _ => Err(ManagedError::UnsupportedFormat),
    }
}
pub(super) fn profile(number: u32) -> Result<Profile> {
    match number {
        1 => Ok(Profile::Source),
        4 => Ok(Profile::Active),
        5 => Ok(Profile::Retired),
        _ => Err(ManagedError::UnsupportedFormat),
    }
}
pub(super) fn quota_check(c: &Connection, expected: u32) -> Result<()> {
    let quota: u32 = c.pragma_query_value(None, "max_page_count", |r| r.get(0))?;
    let pages: u32 = c.pragma_query_value(None, "page_count", |r| r.get(0))?;
    if quota != expected || pages > expected {
        return Err(ManagedError::Limit);
    }
    Ok(())
}
pub(super) fn bind_path(c: &Connection, path: &Path) -> Result<()> {
    schema::local_absolute(path)?;
    let mut statement = c.prepare("PRAGMA database_list")?;
    let mut rows = statement.query([])?;
    let mut main = false;
    let mut temp = false;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let file: String = row.get(2)?;
        match name.as_str() {
            "main" if !main && !file.is_empty() && file.len() <= 4096 => {
                if Path::new(&file).canonicalize()? != path {
                    return Err(ManagedError::SourceMismatch);
                }
                main = true;
            }
            "temp" if !temp && file.is_empty() => {
                let objects: bool = c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM temp.sqlite_schema LIMIT 1)",
                    [],
                    |row| row.get(0),
                )?;
                if objects {
                    return Err(ManagedError::SourceMismatch);
                }
                temp = true;
            }
            _ => return Err(ManagedError::SourceMismatch),
        }
    }
    if !main {
        return Err(ManagedError::SourceMismatch);
    }
    Ok(())
}

#[cfg(test)]
#[path = "world_file_policy_tests.rs"]
mod policy_tests;
