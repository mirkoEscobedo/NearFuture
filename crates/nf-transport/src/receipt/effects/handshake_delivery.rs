//! Actual RR custody around component handshake transitions. FinishedSent alone releases activation.
use super::{
    InboundDelivery, ReceiptBehaviour, ReceiptClientHandshake, ReceiptRepo, ReceiptServerHandshake,
    activation::Activation,
    handshake::ClientPending,
    model::role,
    operation::{Origin, fresh},
};
use crate::{PeerError, receipt::*, records::PeerLimits};
use libp2p::{
    PeerId, Swarm,
    request_response::{InboundRequestId, OutboundRequestId},
    swarm::ConnectionId,
};
use std::time::Instant;
fn returned(
    value: &Option<(ReceiptRecord, Instant, u64)>,
    record: &ReceiptRecord,
) -> Result<(Instant, u64), PeerError> {
    let (expected, created, revision) = value.as_ref().ok_or(PeerError::Replay)?;
    if expected != record {
        return Err(PeerError::Session);
    }
    fresh(*created)?;
    Ok((*created, *revision))
}
impl ReceiptClientHandshake {
    pub(crate) fn guard_outbound(
        &mut self,
        record: &ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let result = (|| {
            let (created, revision) = returned(&self.returned, record)?;
            if self.bound.is_some_and(|b| b != (peer, connection))
                || peer != repo.config().server_pin.peer
            {
                return Err(PeerError::Session);
            }
            repo.require_context(&record.context)?;
            repo.with_current_read(|cut| {
                fresh(created)?;
                if cut.membership.revision != revision {
                    return Err(PeerError::Policy);
                }
                if let Some(ClientPending::Finished { session, .. }) = &self.pending
                    && record.context != session.handshake().context().context
                {
                    return Err(PeerError::Session);
                }
                Ok(())
            })
        })();
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    pub(crate) fn queue_request(
        &mut self,
        record: ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        swarm: &mut Swarm<ReceiptBehaviour>,
        repo: &mut ReceiptRepo,
    ) -> Result<OutboundRequestId, PeerError> {
        self.guard_outbound(&record, peer, connection, repo)?;
        if self.delivery.occupied() {
            return Err(PeerError::Backpressure);
        }
        let (created, revision) = returned(&self.returned, &record)?;
        self.bound = Some((peer, connection));
        let limits = match &self.pending {
            Some(ClientPending::Finished { session, .. }) => session.handshake().context().selected,
            _ => PeerLimits::default(),
        };
        self.delivery.returned(
            &record,
            limits,
            Origin {
                peer,
                connection,
                revision,
                created,
                minimum: SourceMinima {
                    event: nf_contract::identity::EventSeq(0),
                    store_revision: 0,
                    membership_revision: revision,
                },
            },
        )?;
        let id = swarm
            .behaviour_mut()
            .messages
            .send_request(&peer, record.into());
        self.delivery.request(id)?;
        Ok(id)
    }
    pub(crate) fn response_received(
        &mut self,
        request: OutboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        if !self.delivery.matches_request(request, peer, connection) {
            return Err(PeerError::Replay);
        }
        let (record, _) = self.delivery.prepared()?;
        self.guard_outbound(&record, peer, connection, repo)?;
        self.delivery.clear();
        Ok(())
    }
    pub(crate) fn guard_queued(&mut self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
        if !self.delivery.queued() {
            return Ok(());
        }
        let (record, origin) = self.delivery.prepared()?;
        self.guard_outbound(&record, origin.peer, origin.connection, repo)
    }
    pub(crate) fn delivery_failure(
        &mut self,
        request: OutboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> Result<(), PeerError> {
        if !self.delivery.matches_request(request, peer, connection) {
            return Err(PeerError::Replay);
        }
        self.invalidate();
        Ok(())
    }
}
impl ReceiptServerHandshake {
    pub(crate) fn guard_outbound(
        &mut self,
        record: &ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let result = (|| {
            let (created, revision) = returned(&self.returned, record)?;
            if self.bound != Some((peer, connection)) {
                return Err(PeerError::Session);
            }
            repo.require_context(&record.context)?;
            let session = self
                .pending
                .as_ref()
                .map(|p| &p.session)
                .or(self.completed.as_ref())
                .ok_or(PeerError::Replay)?;
            let c = session.handshake().context();
            if record.context != c.context || peer != c.client_peer {
                return Err(PeerError::Session);
            }
            repo.with_current_read(|cut| {
                fresh(created)?;
                if cut.membership.revision != revision {
                    return Err(PeerError::Policy);
                }
                role(cut.membership, c.client_account, c.client_device, peer)?;
                Ok(())
            })
        })();
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    pub(crate) fn queue_response(
        &mut self,
        record: ReceiptRecord,
        active: Option<ReceiptSession>,
        delivery: InboundDelivery,
        swarm: &mut Swarm<ReceiptBehaviour>,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let InboundDelivery {
            peer,
            connection,
            channel,
            request,
        } = delivery;
        if self.delivery.occupied() {
            return Err(PeerError::Backpressure);
        }
        if matches!(record.body, ReceiptBody::Finished(_)) {
            let session = active.ok_or(PeerError::Session)?;
            if !session.active()
                || Some(session.handshake().context_digest()?) != self.finished_digest
            {
                return Err(PeerError::Session);
            }
            self.completed = Some(session);
        } else if active.is_some() {
            return Err(PeerError::Session);
        }
        self.guard_outbound(&record, peer, connection, repo)?;
        let (created, revision) = returned(&self.returned, &record)?;
        let limits = self
            .pending
            .as_ref()
            .map(|p| p.session.handshake().context().selected)
            .or_else(|| {
                self.completed
                    .as_ref()
                    .map(|s| s.handshake().context().selected)
            })
            .ok_or(PeerError::Replay)?;
        self.delivery.returned(
            &record,
            limits,
            Origin {
                peer,
                connection,
                revision,
                created,
                minimum: SourceMinima {
                    event: nf_contract::identity::EventSeq(0),
                    store_revision: 0,
                    membership_revision: revision,
                },
            },
        )?;
        self.delivery.response(request)?;
        if swarm
            .behaviour_mut()
            .messages
            .send_response(channel, record)
            .is_err()
        {
            self.invalidate();
            return Err(PeerError::Offline);
        }
        Ok(())
    }
    pub(in crate::receipt_effects) fn response_sent(
        &mut self,
        request: InboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<Activation>, PeerError> {
        if !self.delivery.matches_response(request, peer, connection) {
            return Err(PeerError::Replay);
        }
        let (record, origin) = self.delivery.prepared()?;
        self.guard_outbound(&record, peer, connection, repo)?;
        self.delivery.clear();
        if matches!(record.body, ReceiptBody::Finished(_)) {
            self.returned = None;
            self.finished_digest = None;
            Ok(self
                .completed
                .take()
                .map(|session| Activation::new(session, origin.created, origin.revision)))
        } else {
            Ok(None)
        }
    }
    pub(crate) fn guard_queued(&mut self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
        if !self.delivery.queued() {
            return Ok(());
        }
        let (record, origin) = self.delivery.prepared()?;
        self.guard_outbound(&record, origin.peer, origin.connection, repo)
    }
    pub(crate) fn delivery_failure(
        &mut self,
        request: InboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> Result<(), PeerError> {
        if !self.delivery.matches_response(request, peer, connection) {
            return Err(PeerError::Replay);
        }
        self.invalidate();
        Ok(())
    }
}
