use super::{ReceiptClientHandshake, ReceiptServerHandshake};
use libp2p::{
    PeerId,
    request_response::{InboundRequestId, OutboundRequestId},
    swarm::ConnectionId,
};
impl ReceiptClientHandshake {
    pub(super) fn owns_request(
        &self,
        id: OutboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> bool {
        self.delivery.matches_request(id, peer, connection)
    }
    pub(super) fn pending_delivery(&self) -> (usize, usize, usize) {
        self.delivery.pending()
    }
}
impl ReceiptServerHandshake {
    pub(super) fn owns_response(
        &self,
        id: InboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> bool {
        self.delivery.matches_response(id, peer, connection)
    }
    pub(super) fn pending_delivery(&self) -> (usize, usize, usize) {
        self.delivery.pending()
    }
}
