use crate::{
    PeerError,
    budget::{ResponseBudget, ResponsePermit},
    records::{Lane, PeerLimits, PeerRecord, encode_body},
};
use libp2p::{PeerId, request_response::InboundRequestId, swarm::ConnectionId};
use std::{collections::BTreeMap, time::Instant};
pub(super) struct Connection {
    pub peer: PeerId,
    pub id: ConnectionId,
    pub opened: Instant,
    pub last: Instant,
    pub limits: PeerLimits,
    budget: ResponseBudget,
    responses: BTreeMap<InboundRequestId, ResponsePermit>,
}
impl Connection {
    pub fn new(
        peer: PeerId,
        id: ConnectionId,
        lane: Lane,
        now: Instant,
    ) -> Result<Self, PeerError> {
        let limits = PeerLimits::default();
        Ok(Self {
            peer,
            id,
            opened: now,
            last: now,
            limits,
            budget: ResponseBudget::new(lane, limits)?,
            responses: BTreeMap::new(),
        })
    }
    pub fn matches(&self, peer: PeerId, id: ConnectionId) -> bool {
        self.peer == peer && self.id == id
    }
    pub fn negotiate(&mut self, lane: Lane, limits: PeerLimits) -> Result<(), PeerError> {
        if !self.responses.is_empty() {
            return Err(PeerError::Backpressure);
        }
        self.budget = ResponseBudget::new(lane, limits)?;
        self.limits = limits;
        Ok(())
    }
    pub fn reserve(
        &mut self,
        lane: Lane,
        id: InboundRequestId,
        record: &PeerRecord,
    ) -> Result<(), PeerError> {
        if self.responses.contains_key(&id) {
            return Err(PeerError::Replay);
        }
        let n = encode_body(record, lane, self.limits)?.len();
        let permit = self.budget.reserve(n)?;
        self.responses.insert(id, permit);
        Ok(())
    }
    pub fn flushed(&mut self, id: InboundRequestId) -> Result<(), PeerError> {
        let permit = self.responses.remove(&id).ok_or(PeerError::Replay)?;
        self.budget.release(permit)
    }
}
