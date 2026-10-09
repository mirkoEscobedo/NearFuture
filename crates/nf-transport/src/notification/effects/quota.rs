use crate::{PeerError, notification::NotifyLimits};
#[derive(Clone, Copy)]
struct Slot {
    serial: u64,
    body: u16,
}
/// Data-only encoded queue accounting, not a grant to sign or transmit.
pub(crate) struct NotifyQuota {
    limits: NotifyLimits,
    owner: [u8; 16],
    next: u64,
    slots: [Option<Slot>; 16],
    closed: bool,
    stats: NotifyQueueStats,
}
/// Opaque exact completion ticket. A wire decoder cannot construct this value.
pub(crate) struct NotifyPermit {
    owner: [u8; 16],
    serial: u64,
    body: u16,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NotifyQueueStats {
    pub items: u16,
    pub body_bytes: u32,
    pub framing_bytes: u32,
    pub highwater_items: u16,
    pub highwater_body_bytes: u32,
    pub highwater_framing_bytes: u32,
}
impl NotifyQuota {
    pub(crate) fn new(limits: NotifyLimits) -> Result<Self, PeerError> {
        limits.validate()?;
        Ok(Self {
            limits,
            owner: crate::session::fresh()?,
            next: 1,
            slots: [None; 16],
            closed: false,
            stats: NotifyQueueStats::default(),
        })
    }
    pub(crate) fn reserve(&mut self, body: usize) -> Result<NotifyPermit, PeerError> {
        if self.closed {
            return Err(PeerError::Session);
        }
        if body > usize::from(self.limits.frame) {
            return Err(PeerError::Limit);
        }
        if body < 128 {
            return Err(PeerError::Malformed);
        }
        let body = u16::try_from(body).map_err(|_| PeerError::Limit)?;
        let total = self
            .stats
            .body_bytes
            .checked_add(u32::from(body))
            .ok_or(PeerError::Limit)?;
        if self.stats.items >= self.limits.queue_items || total > self.limits.queue_bytes {
            return Err(PeerError::Backpressure);
        }
        let serial = self.next;
        self.next = self.next.checked_add(1).ok_or(PeerError::Limit)?;
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(PeerError::Backpressure)?;
        *slot = Some(Slot { serial, body });
        self.stats.items += 1;
        self.stats.body_bytes = total;
        self.stats.framing_bytes = u32::from(self.stats.items) * 4;
        self.stats.highwater_items = self.stats.highwater_items.max(self.stats.items);
        self.stats.highwater_body_bytes = self.stats.highwater_body_bytes.max(total);
        self.stats.highwater_framing_bytes = self
            .stats
            .highwater_framing_bytes
            .max(self.stats.framing_bytes);
        Ok(NotifyPermit {
            owner: self.owner,
            serial,
            body,
        })
    }
    pub(crate) fn complete(&mut self, ticket: NotifyPermit) -> Result<(), PeerError> {
        if self.closed || ticket.owner != self.owner {
            return Err(PeerError::Replay);
        }
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.is_some_and(|s| s.serial == ticket.serial && s.body == ticket.body))
            .ok_or(PeerError::Replay)?;
        *slot = None;
        self.stats.items = self.stats.items.checked_sub(1).ok_or(PeerError::Replay)?;
        self.stats.body_bytes = self
            .stats
            .body_bytes
            .checked_sub(u32::from(ticket.body))
            .ok_or(PeerError::Replay)?;
        self.stats.framing_bytes = u32::from(self.stats.items) * 4;
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn stats(&self) -> NotifyQueueStats {
        self.stats
    }
    pub(crate) fn invalidate(&mut self) {
        self.closed = true;
        self.slots = [None; 16];
        self.stats.items = 0;
        self.stats.body_bytes = 0;
        self.stats.framing_bytes = 0;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct PermitTag {
    owner: [u8; 16],
    serial: u64,
    body: u16,
}
impl NotifyPermit {
    pub(super) fn tag(&self) -> PermitTag {
        PermitTag {
            owner: self.owner,
            serial: self.serial,
            body: self.body,
        }
    }
    pub(super) fn body(&self) -> u16 {
        self.body
    }
}
impl NotifyQuota {
    pub(super) fn owns(&self, p: &NotifyPermit) -> bool {
        !self.closed
            && p.owner == self.owner
            && self
                .slots
                .iter()
                .any(|s| s.is_some_and(|s| s.serial == p.serial && s.body == p.body))
    }
    pub(super) fn tighten(&mut self, limits: NotifyLimits) -> Result<(), PeerError> {
        if self.closed || self.limits.negotiate(limits)? != limits {
            return Err(PeerError::Limit);
        }
        if self.stats.items > limits.queue_items
            || self.stats.body_bytes > limits.queue_bytes
            || self.slots.iter().flatten().any(|s| s.body > limits.frame)
        {
            return Err(PeerError::Backpressure);
        }
        self.limits = limits;
        Ok(())
    }
}
