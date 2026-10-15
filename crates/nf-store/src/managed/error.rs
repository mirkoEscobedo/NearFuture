use crate::StoreError;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedError {
    MissingCatalog,
    UnsupportedFormat,
    Scope,
    SourceMismatch,
    TargetMismatch,
    StaleBackup,
    OwnerMismatch,
    StaleSession,
    Authentication,
    Unauthorized,
    Expired,
    Replay,
    OperationConflict,
    NotDrained,
    Limit,
    AlreadyExists,
    Busy,
    Corrupt,
    DerivationMismatch,
    Quarantined,
    UncertainOutcome,
    ReconciliationRequired,
    Storage(StoreError),
}
impl fmt::Display for ManagedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ManagedError {}
impl From<StoreError> for ManagedError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Busy => Self::Busy,
            StoreError::Quarantined => Self::Quarantined,
            StoreError::UncertainCommit => Self::UncertainOutcome,
            other => Self::Storage(other),
        }
    }
}
impl From<rusqlite::Error> for ManagedError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::from(error).into()
    }
}
impl From<std::io::Error> for ManagedError {
    fn from(_: std::io::Error) -> Self {
        Self::Storage(StoreError::Io)
    }
}
impl From<nf_kernel::Rejection> for ManagedError {
    fn from(_: nf_kernel::Rejection) -> Self {
        Self::Storage(StoreError::InvalidTransition)
    }
}
impl From<nf_identity::model::IdentityError> for ManagedError {
    fn from(_: nf_identity::model::IdentityError) -> Self {
        Self::Authentication
    }
}
pub(super) type Result<T> = std::result::Result<T, ManagedError>;
