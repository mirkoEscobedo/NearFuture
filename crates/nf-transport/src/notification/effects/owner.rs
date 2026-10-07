use super::NotifySession;
use crate::{
    PeerError,
    receipt_effects::{CurrentRead, ReceiptRepo, RemotePrincipal},
};
use libp2p::{PeerId, swarm::ConnectionId};
impl NotifySession {
    /// Synchronous sole-owner borrow; no owned observation is a later permission.
    pub(super) fn with_repo<R>(
        &self,
        repo: &mut ReceiptRepo,
        f: impl for<'a> FnOnce(CurrentRead<'a>) -> Result<R, PeerError>,
    ) -> Result<R, PeerError> {
        let config = repo.config();
        let t = &self.transcript;
        if config.scope != t.context.scope
            || config.ruleset != t.context.ruleset
            || config.content != t.context.content
            || config.server_pin.peer != t.server_peer
            || config.server_pin.account != t.server_account
            || config.server_pin.device != t.server_device
        {
            return Err(PeerError::Policy);
        }
        repo.with_current_read(|cut| {
            self.current(cut.membership)?;
            if cut.local != &self.local
                || cut.device_key.public_key() != self.local.device_key
                || cut.source.membership_revision != self.revision
            {
                return Err(PeerError::Unauthorized);
            }
            f(cut)
        })
    }
    pub(super) fn remote_from_repo(
        &self,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<RemotePrincipal, PeerError> {
        self.with_repo(repo, |_| {
            if peer != self.remote() || connection != self.connection {
                return Err(PeerError::Session);
            }
            let (account, device) = match self.endpoint {
                super::NotifyEndpoint::Server => (
                    self.transcript.client_account,
                    self.transcript.client_device,
                ),
                super::NotifyEndpoint::Client => (
                    self.transcript.server_account,
                    self.transcript.server_device,
                ),
            };
            Ok(RemotePrincipal {
                account,
                device,
                scope: self.transcript.context.scope,
                peer,
            })
        })
    }
}
