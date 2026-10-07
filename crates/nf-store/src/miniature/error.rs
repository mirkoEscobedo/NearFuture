use crate::StoreError;
use nf_identity::model::IdentityError;
use nf_kernel::miniature::MiniatureRejection;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MiniatureStoreError {
    Storage(StoreError),
    Identity(IdentityError),
    Kernel(MiniatureRejection),
    Unauthorized,
    Expired,
    Replay,
    WrongPurpose,
    Unclaimed,
    NoActivity,
    Fenced,
    Entropy,
}
impl From<StoreError> for MiniatureStoreError {
    fn from(e: StoreError) -> Self {
        Self::Storage(e)
    }
}
impl From<rusqlite::Error> for MiniatureStoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(e.into())
    }
}
impl From<IdentityError> for MiniatureStoreError {
    fn from(e: IdentityError) -> Self {
        Self::Identity(e)
    }
}
impl From<MiniatureRejection> for MiniatureStoreError {
    fn from(e: MiniatureRejection) -> Self {
        Self::Kernel(e)
    }
}
impl std::fmt::Display for MiniatureStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for MiniatureStoreError {}
pub(crate) type Result<T> = std::result::Result<T, MiniatureStoreError>;
