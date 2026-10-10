use nf_identity::model::IdentityError;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatStoreError {
    Identity(IdentityError),
    Scope,
    Policy,
    Malformed,
    Limit,
    Signature,
    Replay,
    Expired,
    Conflict,
    Corrupt,
    UnsupportedProfile,
    UnsupportedOperation,
    MissingHistory,
    AlreadyExists,
    StaleBackup,
    Storage,
    Quarantined,
}
pub type Result<T> = std::result::Result<T, ChatStoreError>;
impl From<IdentityError> for ChatStoreError {
    fn from(value: IdentityError) -> Self {
        Self::Identity(value)
    }
}
impl From<rusqlite::Error> for ChatStoreError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}
impl From<crate::StoreError> for ChatStoreError {
    fn from(value: crate::StoreError) -> Self {
        match value {
            crate::StoreError::AlreadyExists => Self::AlreadyExists,
            crate::StoreError::MissingHistory => Self::MissingHistory,
            crate::StoreError::Corrupt => Self::Corrupt,
            crate::StoreError::Limit => Self::Limit,
            crate::StoreError::Quarantined => Self::Quarantined,
            _ => Self::Storage,
        }
    }
}
