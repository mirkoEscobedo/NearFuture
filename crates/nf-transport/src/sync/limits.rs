use super::{ExpectedProfilePins, SyncLane};
use crate::PeerError;
pub const HEADER_BYTES: usize = 126;
pub const CONTROL_HARD_BYTES: usize = 1024;
pub const TRANSFER_HARD_BYTES: usize = 8392;
pub const DOCUMENT_HARD_BYTES: u32 = 2105344;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncLimits {
    pub control_frame: u32,
    pub transfer_frame: u32,
    pub chunk_bytes: u32,
    pub control_queue_bytes: u32,
    pub transfer_queue_bytes: u32,
    pub control_items: u16,
    pub transfer_items: u16,
    pub pending_challenges: u16,
}
impl Default for SyncLimits {
    fn default() -> Self {
        Self {
            control_frame: 1024,
            transfer_frame: 9216,
            chunk_bytes: 8192,
            control_queue_bytes: 16384,
            transfer_queue_bytes: 32768,
            control_items: 4,
            transfer_items: 2,
            pending_challenges: 1,
        }
    }
}
impl SyncLimits {
    pub fn validate(self) -> Result<(), PeerError> {
        if self.control_frame != 1024
            || !(1024..=9216).contains(&self.transfer_frame)
            || !(256..=8192).contains(&self.chunk_bytes)
            || self.chunk_bytes > self.transfer_frame - 200
            || self.control_queue_bytes < self.control_frame
            || self.control_queue_bytes > 16384
            || self.transfer_queue_bytes < self.transfer_frame
            || self.transfer_queue_bytes > 32768
            || !(1..=4).contains(&self.control_items)
            || !(1..=2).contains(&self.transfer_items)
            || self.pending_challenges != 1
        {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub fn negotiate(self, other: Self) -> Result<Self, PeerError> {
        self.validate()?;
        other.validate()?;
        let v = Self {
            control_frame: self.control_frame.min(other.control_frame),
            transfer_frame: self.transfer_frame.min(other.transfer_frame),
            chunk_bytes: self.chunk_bytes.min(other.chunk_bytes),
            control_queue_bytes: self.control_queue_bytes.min(other.control_queue_bytes),
            transfer_queue_bytes: self.transfer_queue_bytes.min(other.transfer_queue_bytes),
            control_items: self.control_items.min(other.control_items),
            transfer_items: self.transfer_items.min(other.transfer_items),
            pending_challenges: self.pending_challenges.min(other.pending_challenges),
        };
        v.validate()?;
        Ok(v)
    }
    pub fn document_maximum(self) -> Result<u32, PeerError> {
        self.validate()?;
        Ok(DOCUMENT_HARD_BYTES.min(self.chunk_bytes.checked_mul(257).ok_or(PeerError::Limit)?))
    }
    pub(super) fn document(self, total: u32, count: u16) -> Result<(), PeerError> {
        if total == 0
            || total > self.document_maximum()?
            || count == 0
            || count > 257
            || total.div_ceil(self.chunk_bytes) != u32::from(count)
        {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub(super) fn chunk(
        self,
        total: u32,
        count: u16,
        index: u16,
        n: usize,
    ) -> Result<(), PeerError> {
        self.document(total, count)?;
        if n > self.chunk_bytes as usize {
            return Err(PeerError::Limit);
        }
        if index >= count {
            return Err(PeerError::Malformed);
        }
        let expected = if index + 1 == count {
            total - u32::from(index) * self.chunk_bytes
        } else {
            self.chunk_bytes
        };
        if n == 0 || n != expected as usize {
            return Err(PeerError::Malformed);
        }
        Ok(())
    }
}
/// Expected resource and namespace data, not observed backend or negotiation proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyncWirePolicy {
    pub lane: SyncLane,
    pub limits: SyncLimits,
    pub pins: ExpectedProfilePins,
}
impl SyncWirePolicy {
    pub fn validate(self) -> Result<(), PeerError> {
        self.limits.validate()
    }
    pub fn frame_maximum(self) -> Result<usize, PeerError> {
        self.validate()?;
        Ok(match self.lane {
            SyncLane::Control => CONTROL_HARD_BYTES,
            SyncLane::Transfer => TRANSFER_HARD_BYTES.min(self.limits.transfer_frame as usize),
        })
    }
}
