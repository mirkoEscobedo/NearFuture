use super::NotifyLimits;
use crate::PeerError;
impl NotifyLimits {
    pub fn validate(self) -> Result<(), PeerError> {
        if !(768..=1024).contains(&self.frame)
            || self.queue_bytes < u32::from(self.frame)
            || self.queue_bytes > 16384
            || !(1..=16).contains(&self.queue_items)
            || !(1..=8).contains(&self.rate)
            || !(1..=8).contains(&self.burst)
            || self.pending != 1
        {
            return Err(PeerError::Limit);
        }
        Ok(())
    }
    pub fn negotiate(self, other: Self) -> Result<Self, PeerError> {
        self.validate()?;
        other.validate()?;
        let selected = Self {
            frame: self.frame.min(other.frame),
            queue_bytes: self.queue_bytes.min(other.queue_bytes),
            queue_items: self.queue_items.min(other.queue_items),
            rate: self.rate.min(other.rate),
            burst: self.burst.min(other.burst),
            pending: self.pending.min(other.pending),
        };
        selected.validate()?;
        Ok(selected)
    }
}
