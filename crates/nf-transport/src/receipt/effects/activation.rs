//! Genuine handshake completion retains its original call time and admitted epoch.
use crate::receipt::ReceiptSession;
use std::time::Instant;
pub(super) struct Activation {
    pub(super) session: ReceiptSession,
    pub(super) created: Instant,
    pub(super) revision: u64,
}
impl Activation {
    pub(super) fn new(session: ReceiptSession, created: Instant, revision: u64) -> Self {
        Self {
            session,
            created,
            revision,
        }
    }
    pub(super) fn into_session(self) -> ReceiptSession {
        self.session
    }
}
