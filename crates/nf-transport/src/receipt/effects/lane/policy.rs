use super::super::model::role;
use super::super::{activation::Activation, operation::fresh};
use super::*;
use crate::auth::HandshakeContext;
use nf_contract::identity::{AccountId, DeviceId};
pub(super) struct ActivePolicy {
    revision: u64,
    client: (AccountId, DeviceId, PeerId),
    server: (AccountId, DeviceId, PeerId),
    context: crate::records::PeerContext,
}
impl ActivePolicy {
    pub(super) fn capture(
        activation: &Activation,
        repo: &mut ReceiptRepo,
    ) -> Result<Self, PeerError> {
        let c: &HandshakeContext = activation.session.handshake().context();
        repo.require_context(&c.context)?;
        repo.with_current_read(|cut| {
            fresh(activation.created)?;
            if cut.membership.revision != activation.revision
                || cut.membership.owner != c.server_account
            {
                return Err(PeerError::Policy);
            }
            for (account, device, peer) in [
                (c.client_account, c.client_device, c.client_peer),
                (c.server_account, c.server_device, c.server_peer),
            ] {
                role(cut.membership, account, device, peer)?;
            }
            Ok(Self {
                revision: activation.revision,
                client: (c.client_account, c.client_device, c.client_peer),
                server: (c.server_account, c.server_device, c.server_peer),
                context: c.context,
            })
        })
    }
    fn check(&self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
        repo.require_context(&self.context)?;
        repo.with_current_read(|cut| {
            if cut.membership.revision != self.revision || cut.membership.owner != self.server.0 {
                return Err(PeerError::Policy);
            }
            for (account, device, peer) in [self.client, self.server] {
                role(cut.membership, account, device, peer)?;
            }
            Ok(())
        })
    }
}
impl ReceiptLane {
    pub(super) fn guard(&mut self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
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
        if let Some(active) = &self.active {
            active.check(repo)?;
        }
        match &mut self.endpoint {
            Endpoint::Server {
                handshake,
                operation,
            } => {
                handshake.guard_queued(repo)?;
                if let Some(operation) = operation {
                    operation.guard_queued(repo)?;
                }
            }
            Endpoint::Client {
                handshake,
                operation,
                ..
            } => {
                handshake.guard_queued(repo)?;
                if let Some(operation) = operation {
                    operation.guard_queued(repo)?;
                }
            }
        }
        Ok(())
    }
}
