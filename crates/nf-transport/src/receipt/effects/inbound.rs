//! An actual backend response channel and its immutable physical routing tuple.
use crate::receipt::ReceiptRecord;
use libp2p::{
    PeerId,
    request_response::{InboundRequestId, ResponseChannel},
    swarm::ConnectionId,
};
pub(crate) struct InboundDelivery {
    pub peer: PeerId,
    pub connection: ConnectionId,
    pub channel: ResponseChannel<ReceiptRecord>,
    pub request: InboundRequestId,
}
