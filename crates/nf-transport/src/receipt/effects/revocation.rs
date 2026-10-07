use super::ReceiptRepo;
use crate::PeerError;
use nf_identity::model::DeviceRevocation;
impl ReceiptRepo {
    /// Trusted local signed policy command, never a receipt/notification wire operation.
    /// No Store/CAS callback or key getter exists. Every later stage reloads the new SQL state.
    pub fn revoke_trusted(
        &mut self,
        change: DeviceRevocation,
        signature: [u8; 64],
    ) -> Result<u64, PeerError> {
        self.with_current_read(|_| Ok(()))?;
        let state = nf_identity::persistence::revoke_persisted(
            &mut self.store,
            self.config.scope,
            &change,
            &signature,
        )
        .map_err(|_| PeerError::Unauthorized)?;
        Ok(state.revision)
    }
}
