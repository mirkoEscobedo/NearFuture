//! Conservative encoded-body queue accounting. Object/parser/backend memory is a separate finite residual.
use crate::{
    PeerError,
    records::{Lane, PeerLimits},
    session::fresh,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResponsePermit {
    owner: [u8; 16],
    ticket: u64,
}
pub struct ResponseBudget {
    owner: [u8; 16],
    next: u64,
    entries: BTreeMap<u64, usize>,
    bytes: usize,
    frame: usize,
    max_bytes: usize,
    max_items: usize,
}
impl ResponseBudget {
    pub fn new(lane: Lane, limits: PeerLimits) -> Result<Self, PeerError> {
        limits.validate()?;
        let (max_bytes, max_items) = match lane {
            Lane::Control => (limits.control_queue_bytes, limits.control_items),
            Lane::Bulk => (limits.bulk_queue_bytes, limits.bulk_items),
        };
        Ok(Self {
            owner: fresh()?,
            next: 0,
            entries: BTreeMap::new(),
            bytes: 0,
            frame: limits.frame(lane),
            max_bytes: max_bytes as usize,
            max_items: max_items as usize,
        })
    }
    pub fn reserve(&mut self, encoded_body: usize) -> Result<ResponsePermit, PeerError> {
        if encoded_body == 0 || encoded_body > self.frame {
            return Err(PeerError::Limit);
        }
        let bytes = self
            .bytes
            .checked_add(encoded_body)
            .ok_or(PeerError::Limit)?;
        if self.entries.len() >= self.max_items || bytes > self.max_bytes {
            return Err(PeerError::Backpressure);
        }
        let next = self.next.checked_add(1).ok_or(PeerError::Limit)?;
        let permit = ResponsePermit {
            owner: self.owner,
            ticket: self.next,
        };
        self.entries.insert(self.next, encoded_body);
        self.next = next;
        self.bytes = bytes;
        Ok(permit)
    }
    pub fn release(&mut self, p: ResponsePermit) -> Result<(), PeerError> {
        if p.owner != self.owner {
            return Err(PeerError::Replay);
        }
        let n = self.entries.remove(&p.ticket).ok_or(PeerError::Replay)?;
        self.bytes -= n;
        Ok(())
    }
    pub fn pending(&self) -> (usize, usize) {
        (self.entries.len(), self.bytes)
    }
}
