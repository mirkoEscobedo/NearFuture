use super::{error::ManagedError, reconcile::ReconciliationHandle, source_owner::SourceOwner};
use crate::Store;
use nf_contract::identity::JobId;
use std::fmt;

pub struct AdoptionFailure {
    pub(super) error: ManagedError,
    pub(super) source: Option<Store>,
    pub(super) reconciliation: Option<ReconciliationHandle>,
}
impl AdoptionFailure {
    pub fn error(&self) -> ManagedError { self.error }
    pub fn into_parts(self) -> (ManagedError, Option<Store>, Option<ReconciliationHandle>) {
        (self.error, self.source, self.reconciliation)
    }
}
impl fmt::Debug for AdoptionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("AdoptionFailure").field(&self.error).finish() }
}
pub type AdoptionOutcome = AdoptionFailure;
pub struct PrecommitFailure<T> {
    pub(super) error: ManagedError,
    pub(super) owner: Option<T>,
    pub(super) reconciliation: Option<ReconciliationHandle>,
}
impl<T> PrecommitFailure<T> {
    pub fn error(&self) -> ManagedError { self.error }
    pub fn into_parts(self) -> (ManagedError, Option<T>, Option<ReconciliationHandle>) {
        (self.error, self.owner, self.reconciliation)
    }
}
impl<T> fmt::Debug for PrecommitFailure<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("PrecommitFailure").field(&self.error).finish() }
}
pub enum UpgradeOutcome {
    SealedReturned { error: ManagedError, source: super::sealed::ManagedDrainedSource },
    SourceReturned { error: ManagedError, source: SourceOwner },
    Reconciliation { error: ManagedError, handle: ReconciliationHandle },
}
impl UpgradeOutcome {
    pub fn error(&self) -> ManagedError {
        match self { Self::SealedReturned { error, .. } | Self::SourceReturned { error, .. }
            | Self::Reconciliation { error, .. } => *error }
    }
}
impl fmt::Debug for UpgradeOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("UpgradeOutcome").field(&self.error()).finish() }
}
#[derive(Debug)]
pub enum SettlementFailure { Paused(Vec<JobId>), Refused(ManagedError) }
impl SettlementFailure {
    pub fn missing_jobs(&self) -> &[JobId] {
        match self { Self::Paused(jobs) => jobs, Self::Refused(_) => &[] }
    }
    pub fn error(&self) -> Option<ManagedError> {
        match self { Self::Paused(_) => None, Self::Refused(error) => Some(*error) }
    }
}
impl From<ManagedError> for SettlementFailure {
    fn from(error: ManagedError) -> Self { Self::Refused(error) }
}
