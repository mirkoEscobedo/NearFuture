use nf_identity::model::IdentityError;
use nf_kernel::supplies::SuppliesRejection;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SuppliesStoreError {
    Identity(IdentityError),
    Storage(crate::StoreError),
    Rejected(SuppliesRejection),
    Compact(super::compact::CompactFailure),
    Corrupt,
    UnsupportedProfile,
    StaleBackup,
    Replay,
    Expired,
    Entropy,
}
pub(super) type Result<T> = std::result::Result<T, SuppliesStoreError>;
impl From<crate::StoreError> for SuppliesStoreError {
    fn from(value: crate::StoreError) -> Self {
        Self::Storage(value)
    }
}
impl From<rusqlite::Error> for SuppliesStoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Storage(value.into())
    }
}
impl From<IdentityError> for SuppliesStoreError {
    fn from(value: IdentityError) -> Self {
        Self::Identity(value)
    }
}
impl From<SuppliesRejection> for SuppliesStoreError {
    fn from(value: SuppliesRejection) -> Self {
        Self::Rejected(value)
    }
}
