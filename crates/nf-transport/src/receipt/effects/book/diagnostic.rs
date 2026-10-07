use crate::PeerError;
use nf_identity::private_storage::PrivateFailure;
/// Private propagation only: no receipt-facing public error contract changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BookFailure {
    error: PeerError,
    storage: Option<PrivateFailure>,
}
impl BookFailure {
    pub(crate) fn peer_error(self) -> PeerError {
        self.error
    }
    #[cfg(test)]
    pub(crate) fn storage_failure(self) -> Option<PrivateFailure> {
        self.storage
    }
    pub(crate) fn storage(failure: PrivateFailure) -> Self {
        Self {
            error: PeerError::Storage,
            storage: Some(failure),
        }
    }
}
impl From<PeerError> for BookFailure {
    fn from(error: PeerError) -> Self {
        Self {
            error,
            storage: None,
        }
    }
}
pub(crate) type BookResult<T> = Result<T, BookFailure>;
