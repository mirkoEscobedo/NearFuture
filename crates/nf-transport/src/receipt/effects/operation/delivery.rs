//! One private returned record, then one exact backend custody. No public release/grant setter.
use crate::{
    PeerError,
    receipt::{ReceiptRecord, SourceMinima, encode_body},
    records::PeerLimits,
};
use libp2p::{
    PeerId,
    request_response::{InboundRequestId, OutboundRequestId},
    swarm::ConnectionId,
};
use sha2::{Digest, Sha256};
use std::time::Instant;
#[derive(Clone, Copy)]
pub(in crate::receipt_effects) struct Origin {
    pub peer: PeerId,
    pub connection: ConnectionId,
    pub revision: u64,
    pub created: Instant,
    pub minimum: SourceMinima,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Backend {
    Prepared,
    Response(InboundRequestId),
    Request(OutboundRequestId),
}
struct Entry {
    record: ReceiptRecord,
    digest: [u8; 32],
    origin: Origin,
    backend: Backend,
    bytes: usize,
}
#[derive(Default)]
pub(in crate::receipt_effects) struct Delivery {
    entry: Option<Entry>,
}
impl Delivery {
    pub fn returned(
        &mut self,
        record: &ReceiptRecord,
        limits: PeerLimits,
        origin: Origin,
    ) -> Result<(), PeerError> {
        if self.entry.is_some() {
            return Err(PeerError::Backpressure);
        }
        let bytes = encode_body(record, limits)?;
        if limits.control_items == 0 || bytes.len() > limits.control_queue_bytes as usize {
            return Err(PeerError::Backpressure);
        }
        self.entry = Some(Entry {
            record: record.clone(),
            digest: Sha256::digest(&bytes).into(),
            origin,
            backend: Backend::Prepared,
            bytes: bytes.len(),
        });
        Ok(())
    }
    pub fn occupied(&self) -> bool {
        self.entry.is_some()
    }
    pub fn queued(&self) -> bool {
        self.entry
            .as_ref()
            .is_some_and(|e| e.backend != Backend::Prepared)
    }
    pub fn prepared(&self) -> Result<(ReceiptRecord, Origin), PeerError> {
        let e = self.entry.as_ref().ok_or(PeerError::Replay)?;
        Ok((e.record.clone(), e.origin))
    }
    pub fn check(
        &self,
        record: &ReceiptRecord,
        peer: PeerId,
        id: ConnectionId,
        limits: PeerLimits,
    ) -> Result<Origin, PeerError> {
        let e = self.entry.as_ref().ok_or(PeerError::Replay)?;
        super::common::fresh(e.origin.created)?;
        if peer != e.origin.peer
            || id != e.origin.connection
            || <[u8; 32]>::from(Sha256::digest(encode_body(record, limits)?)) != e.digest
        {
            return Err(PeerError::Session);
        }
        Ok(e.origin)
    }
    pub fn response(&mut self, id: InboundRequestId) -> Result<(), PeerError> {
        self.bind(Backend::Response(id))
    }
    pub fn request(&mut self, id: OutboundRequestId) -> Result<(), PeerError> {
        self.bind(Backend::Request(id))
    }
    fn bind(&mut self, kind: Backend) -> Result<(), PeerError> {
        let e = self.entry.as_mut().ok_or(PeerError::Replay)?;
        if e.backend != Backend::Prepared {
            return Err(PeerError::Backpressure);
        }
        e.backend = kind;
        Ok(())
    }
    pub fn matches_response(
        &self,
        id: InboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> bool {
        self.matches(Backend::Response(id), peer, connection)
    }
    pub fn matches_request(
        &self,
        id: OutboundRequestId,
        peer: PeerId,
        connection: ConnectionId,
    ) -> bool {
        self.matches(Backend::Request(id), peer, connection)
    }
    fn matches(&self, backend: Backend, peer: PeerId, connection: ConnectionId) -> bool {
        self.entry.as_ref().is_some_and(|e| {
            e.backend == backend && e.origin.peer == peer && e.origin.connection == connection
        })
    }
    pub fn clear_prepared(&mut self) -> Result<(), PeerError> {
        if self.queued() {
            return Err(PeerError::Backpressure);
        }
        self.clear();
        Ok(())
    }
    pub fn clear(&mut self) {
        self.entry = None;
    }
    pub fn pending(&self) -> (usize, usize, usize) {
        self.entry.as_ref().map_or((0, 0, 0), |e| (1, e.bytes, 4))
    }
}
