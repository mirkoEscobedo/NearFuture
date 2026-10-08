use crate::supplies::SuppliesStoreError;
use nf_kernel::trade::TradeRejection;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeStoreError {
    Supplies(SuppliesStoreError),
    Rejected(TradeRejection),
    UnsupportedOperation,
}
pub(super) type Result<T> = std::result::Result<T, TradeStoreError>;
impl From<SuppliesStoreError> for TradeStoreError {
    fn from(value: SuppliesStoreError) -> Self {
        Self::Supplies(value)
    }
}
impl From<TradeRejection> for TradeStoreError {
    fn from(value: TradeRejection) -> Self {
        Self::Rejected(value)
    }
}
impl From<rusqlite::Error> for TradeStoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Supplies(value.into())
    }
}
impl From<crate::StoreError> for TradeStoreError {
    fn from(value: crate::StoreError) -> Self {
        Self::Supplies(value.into())
    }
}
