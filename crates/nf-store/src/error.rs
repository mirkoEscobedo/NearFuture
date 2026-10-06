use std::fmt;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreError {
    Io,
    Busy,
    Full,
    Corrupt,
    UnsupportedSchema,
    MissingHistory,
    AlreadyExists,
    Scope,
    StaleBackup,
    RequestConflict,
    InvalidTransition,
    Limit,
    Backpressure,
    Quarantined,
    UncertainCommit,
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StoreError {}
impl From<nf_kernel::Rejection> for StoreError {
    fn from(_: nf_kernel::Rejection) -> Self {
        Self::InvalidTransition
    }
}
impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        use rusqlite::{Error, ErrorCode};
        match error {
            Error::SqliteFailure(error, _) => match error.code {
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => Self::Busy,
                ErrorCode::DiskFull => Self::Full,
                ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => Self::Corrupt,
                ErrorCode::TooBig => Self::Limit,
                _ => Self::Io,
            },
            _ => Self::Corrupt,
        }
    }
}
pub(crate) type Result<T> = std::result::Result<T, StoreError>;
