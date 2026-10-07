use super::NotifyPermit;
use crate::notification::NotifyRecord;
/// Immutable prepared record and its sole owned encoded-queue reservation.
/// A decoder cannot construct delivery custody; backend ownership must match completion.
pub(crate) struct NotifyEmission {
    pub(super) record: NotifyRecord,
    pub(super) permit: NotifyPermit,
}
impl NotifyEmission {
    pub(crate) fn record(&self) -> &NotifyRecord {
        &self.record
    }
    pub(crate) fn into_parts(self) -> (NotifyRecord, NotifyPermit) {
        (self.record, self.permit)
    }
}
