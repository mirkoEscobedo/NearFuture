use super::super::{InboundDelivery, ReceiptBehaviour, ReceiptRepo};
use super::{ReceiptOperationClient, ReceiptOperationServer};
use crate::{PeerError, receipt::*};
use libp2p::{
    PeerId, Swarm,
    request_response::{InboundRequestId, OutboundRequestId},
    swarm::ConnectionId,
};
impl ReceiptOperationServer {
    pub(crate) fn guard_outbound(
        &mut self,
        record: &ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let result = (|| {
            let origin = self.delivery.check(
                record,
                peer,
                connection,
                self.session.handshake().context().selected,
            )?;
            repo.require_context(&record.context)?;
            repo.with_current_read(|cut| {
                super::fresh(origin.created)?;
                if cut.membership.revision != origin.revision {
                    return Err(PeerError::Policy);
                }
                self.session
                    .admit_record(record, peer, connection, cut.membership)?;
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
        self.guard_outbound(&record, peer, connection, repo)?;
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
    pub(crate) fn response_sent(
        &mut self,
        request: InboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        if !self.delivery.matches_response(request, peer, connection) {
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
    pub(crate) fn pending_delivery(&self) -> (usize, usize, usize) {
        self.delivery.pending()
    }
}
impl ReceiptOperationClient {
    pub(crate) fn guard_outbound(
        &mut self,
        record: &ReceiptRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let result = (|| {
            self.require_query_live()?;
            let origin = self.delivery.check(
                record,
                peer,
                connection,
                self.session.handshake().context().selected,
            )?;
            repo.require_context(&record.context)?;
            if !origin
                .minimum
                .admits(repo.protected_minimum(&self.original)?)
            {
                return Err(PeerError::Policy);
            }
            repo.with_current_read(|cut| {
                super::fresh(origin.created)?;
                self.require_query_live()?;
                if cut.membership.revision != origin.revision {
                    return Err(PeerError::Policy);
                }
                self.session
                    .admit_record(record, peer, connection, cut.membership)?;
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
        if self.delivery.queued() {
            return Err(PeerError::Backpressure);
        }
        self.require_query_live()?;
        let request = swarm
            .behaviour_mut()
            .messages
            .send_request(&peer, record.into());
        self.delivery.request(request)?;
        Ok(request)
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
    pub(crate) fn pending_delivery(&self) -> (usize, usize, usize) {
        self.delivery.pending()
    }
}

impl ReceiptOperationClient {
    pub(in crate::receipt_effects) fn owns_request(
        &self,
        id: OutboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> bool {
        self.delivery.matches_request(id, peer, connection)
    }
}
impl ReceiptOperationServer {
    pub(in crate::receipt_effects) fn owns_response(
        &self,
        id: InboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> bool {
        self.delivery.matches_response(id, peer, connection)
    }
}
