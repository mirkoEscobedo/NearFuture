//! Trusted receiver-local monotonic seam. No sender fields or persisted wall-clock values.
use std::time::{Duration, Instant};
pub trait ReceiverClock: Send + Sync {
    fn now(&self) -> Duration;
}
pub struct MonotonicReceiverClock {
    started: Instant,
}
impl Default for MonotonicReceiverClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}
impl ReceiverClock for MonotonicReceiverClock {
    fn now(&self) -> Duration {
        self.started.elapsed()
    }
}
