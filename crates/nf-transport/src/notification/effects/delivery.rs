use super::{NotifyClientHandshake, NotifyEmission, NotifyServerHandshake, NotifySession};
use crate::{PeerError, notification::NotifyRecord, receipt_effects::ReceiptRepo};
use libp2p::{PeerId, swarm::ConnectionId};
impl NotifyClientHandshake {
    pub(super) fn initial_cut(
        &self,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        if let Some(session) = &self.pending {
            session.remote_from_repo(peer, connection, repo)?;
            return session.fresh_setup();
        }
        let (_, created) = self.initial.ok_or(PeerError::Replay)?;
        if std::time::Instant::now()
            .checked_duration_since(created)
            .is_none_or(|d| d >= std::time::Duration::from_secs(5))
        {
            return Err(PeerError::Replay);
        }
        let c = repo.config();
        if peer != self.pin.peer
            || self.sent_connection.is_some_and(|a| a != connection)
            || c.scope != self.policy.scope
            || c.ruleset != self.policy.ruleset
            || c.content != self.policy.content
            || c.server_pin.peer != self.pin.peer
            || c.server_pin.account != self.pin.account
            || c.server_pin.device != self.pin.device
        {
            return Err(PeerError::Session);
        }
        repo.with_current_read(|cut| {
            super::elapsed_cut(created, std::time::Duration::from_secs(5))?;
            if cut.local != &self.local
                || cut.membership.revision
                    < self
                        .policy
                        .minimum_membership
                        .max(self.pin.minimum_membership)
            {
                Err(PeerError::Unauthorized)
            } else {
                Ok(())
            }
        })
    }
    pub(crate) fn prepare_output(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyEmission, PeerError> {
        self.initial_cut(peer, connection, repo)?;
        if let Some(session) = &mut self.pending {
            return session.prepare_setup_output(record, peer, connection, repo);
        }
        self.sent_connection = Some(connection);
        self.flow.as_mut().ok_or(PeerError::Replay)?.output(record)
    }
    pub(crate) fn complete_emission(
        &mut self,
        emission: NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.initial_cut(peer, connection, repo)?;
        if let Some(session) = &mut self.pending {
            return session.complete_setup_emission(emission, peer, connection, repo);
        }
        self.flow
            .as_mut()
            .ok_or(PeerError::Replay)?
            .finish_emission(emission)
            .map(|_| ())
    }
}
impl NotifyServerHandshake {
    pub(crate) fn prepare_output(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyEmission, PeerError> {
        self.pending
            .as_mut()
            .ok_or(PeerError::Replay)?
            .prepare_setup_output(record, peer, connection, repo)
    }
    pub(crate) fn complete_emission(
        &mut self,
        emission: NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.pending
            .as_mut()
            .ok_or(PeerError::Replay)?
            .complete_setup_emission(emission, peer, connection, repo)
    }
}
impl NotifySession {
    pub(in crate::notification_effects) fn prepare_setup_output(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyEmission, PeerError> {
        self.remote_from_repo(peer, connection, repo)?;
        self.fresh_setup()?;
        if record.context != self.context() {
            return Err(PeerError::Session);
        }
        self.flow.output(record)
    }
    pub(in crate::notification_effects) fn complete_setup_emission(
        &mut self,
        emission: NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.remote_from_repo(peer, connection, repo)?;
        self.fresh_setup()?;
        self.flow.finish_emission(emission).map(|_| ())
    }
    pub(crate) fn prepare_output(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyEmission, PeerError> {
        self.remote_from_repo(peer, connection, repo)?;
        if record.context != self.context() {
            return Err(PeerError::Session);
        }
        self.flow.output(record)
    }
    #[cfg(test)]
    pub(crate) fn complete_emission(
        &mut self,
        emission: NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.remote_from_repo(peer, connection, repo)?;
        self.flow.finish_emission(emission).map(|_| ())
    }
}
