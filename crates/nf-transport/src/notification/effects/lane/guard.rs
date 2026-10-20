use super::*;
impl NotifyLane {
    pub(super) fn guard(&self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
        let c = repo.config();
        let b = self.binding;
        if c.scope != b.scope
            || c.ruleset != b.ruleset
            || c.content != b.content
            || c.local_account != b.local_account
            || c.local_device != b.local_device
            || c.minimum_membership != b.minimum_membership
            || c.server_pin.peer != b.server_pin.peer
            || c.server_pin.account != b.server_pin.account
            || c.server_pin.device != b.server_pin.device
            || c.server_pin.minimum_membership != b.server_pin.minimum_membership
            || repo.local_public() != &self.local
        {
            return Err(PeerError::Scope);
        }
        repo.with_current_read(|_| Ok(()))?;
        let Some((peer, id)) = self.connection else {
            return Ok(());
        };
        let session = match &self.endpoint {
            Endpoint::Server {
                handshake,
                session,
                broker,
            } => broker
                .as_ref()
                .map(|b| &b.session)
                .or(session.as_ref())
                .or(handshake.as_ref().and_then(|h| h.pending.as_ref())),
            Endpoint::Client {
                handshake,
                subscriber,
                ..
            } => subscriber
                .as_ref()
                .map(|s| &s.session)
                .or(handshake.as_ref().and_then(|h| h.pending.as_ref())),
        };
        if let Some(s) = session {
            s.remote_from_repo(peer, id, repo)?;
        }
        Ok(())
    }
}
