use crate::IpcError;
use std::collections::VecDeque;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lane {
    Control,
    Bulk,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueueLimits {
    pub control_bytes: usize,
    pub bulk_bytes: usize,
    pub control_items: usize,
    pub bulk_items: usize,
}
impl QueueLimits {
    fn validate(self) -> Result<(), IpcError> {
        let bytes = self
            .control_bytes
            .checked_add(self.bulk_bytes)
            .ok_or(IpcError::Limit)?;
        let items = self
            .control_items
            .checked_add(self.bulk_items)
            .ok_or(IpcError::Limit)?;
        if self.control_bytes == 0
            || self.bulk_bytes == 0
            || self.control_items == 0
            || self.bulk_items == 0
            || bytes > 16_777_216
            || items > 256
        {
            return Err(IpcError::Limit);
        }
        Ok(())
    }
}
struct Queue {
    items: VecDeque<Vec<u8>>,
    bytes: usize,
    byte_limit: usize,
    item_limit: usize,
}
impl Queue {
    fn new(bytes: usize, items: usize) -> Self {
        Self {
            items: VecDeque::new(),
            bytes: 0,
            byte_limit: bytes,
            item_limit: items,
        }
    }
    fn push(&mut self, body: Vec<u8>) -> Result<(), IpcError> {
        if body.is_empty() {
            return Err(IpcError::Malformed);
        }
        let size = self.bytes.checked_add(body.len()).ok_or(IpcError::Limit)?;
        if size > self.byte_limit || self.items.len() >= self.item_limit {
            return Err(IpcError::Backpressure);
        }
        self.bytes = size;
        self.items.push_back(body);
        Ok(())
    }
    fn pop(&mut self) -> Option<Vec<u8>> {
        let body = self.items.pop_front()?;
        self.bytes -= body.len();
        Some(body)
    }
    fn clear(&mut self) {
        self.items.clear();
        self.bytes = 0;
    }
}
/// Independent reserved lanes. The owner supplies synchronization outside the game thread.
/// Failed admission changes nothing; callers retain authoritative request identity for retry.
pub struct BoundedQueues {
    control: Queue,
    bulk: Queue,
}
impl BoundedQueues {
    pub fn new(limits: QueueLimits) -> Result<Self, IpcError> {
        limits.validate()?;
        Ok(Self {
            control: Queue::new(limits.control_bytes, limits.control_items),
            bulk: Queue::new(limits.bulk_bytes, limits.bulk_items),
        })
    }
    pub fn try_push(&mut self, lane: Lane, body: Vec<u8>) -> Result<(), IpcError> {
        match lane {
            Lane::Control => self.control.push(body),
            Lane::Bulk => self.bulk.push(body),
        }
    }
    pub fn pop(&mut self) -> Option<(Lane, Vec<u8>)> {
        self.control
            .pop()
            .map(|v| (Lane::Control, v))
            .or_else(|| self.bulk.pop().map(|v| (Lane::Bulk, v)))
    }
    pub fn bytes(&self) -> usize {
        self.control.bytes + self.bulk.bytes
    }
    pub fn clear(&mut self) {
        self.control.clear();
        self.bulk.clear();
    }
}
