use super::ReceiptRepo;
use crate::{PeerError, receipt::OriginalReceipt};
use nf_kernel::{CommittedBatch, state_hash};
use nf_store::{DurableAck, RequestStatus, Store};
/// One explicit trusted local command sealed from an actual already prepared operation.
/// There is no wire constructor, Store callback, Clone or production network write route.
pub struct TrustedPreparedCommit {
    batch: CommittedBatch,
    original: OriginalReceipt,
    revision: u64,
    before: [u8; 32],
}
impl TrustedPreparedCommit {
    pub fn seal(
        store: &Store,
        original: OriginalReceipt,
        batch: CommittedBatch,
    ) -> Result<Self, PeerError> {
        let before = state_hash(store.world()).map_err(|_| PeerError::Storage)?;
        if before != batch.before_hash || store.pending().is_none_or(|p| p.input_hash() != before) {
            return Err(PeerError::Replay);
        }
        pending(store, &original)?;
        if !batch.admitted.iter().any(|i| {
            i.request == original.request()
                && i.operation == original.operation()
                && i.actor == original.original().account_id
        }) {
            return Err(PeerError::Unauthorized);
        }
        Ok(Self {
            batch,
            original,
            revision: store.revision(),
            before,
        })
    }
}
impl ReceiptRepo {
    pub fn commit_trusted_prepared(
        &mut self,
        command: TrustedPreparedCommit,
    ) -> Result<DurableAck, PeerError> {
        let pin = self.config.server_pin;
        self.with_current_read(|cut| {
            if cut.local.account != pin.account || cut.local.device != pin.device {
                return Err(PeerError::Unauthorized);
            }
            Ok(())
        })?;
        if self.store.revision() != command.revision
            || state_hash(self.store.world()).map_err(|_| PeerError::Storage)? != command.before
        {
            return Err(PeerError::Replay);
        }
        pending(&self.store, &command.original)?;
        self.store
            .commit(&command.batch)
            .map_err(|_| PeerError::Storage)
    }
}
fn pending(store: &Store, original: &OriginalReceipt) -> Result<(), PeerError> {
    let o = original.original();
    let r = store
        .query_bound(
            original.request(),
            o.account_id,
            o.device_id,
            nf_identity::model::Scope {
                universe: o.universe_id,
                history: o.history_id,
            },
        )
        .map_err(|_| PeerError::Unauthorized)?
        .ok_or(PeerError::Unauthorized)?;
    if r.binding_digest != original.binding_digest()
        || !matches!(r.status,RequestStatus::Pending {operation} if operation==original.operation())
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
