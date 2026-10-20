//! Private monotonic refusal cut plus a cooperative wakeup; neither grants authority.
use std::{
    future::Future,
    pin::Pin,
    task::Context,
    time::{Duration, Instant},
};
pub(super) struct Deadline {
    expires: Instant,
    sleep: Pin<Box<tokio::time::Sleep>>,
}
impl Deadline {
    pub(super) fn after(duration: Duration) -> Self {
        Self::at(Instant::now() + duration)
    }
    pub(super) fn at(expires: Instant) -> Self {
        Self {
            expires,
            sleep: Box::pin(tokio::time::sleep_until(tokio::time::Instant::from_std(
                expires,
            ))),
        }
    }
    pub(super) fn end(&self) -> Instant {
        self.expires
    }
    pub(super) fn expired_now(&self) -> bool {
        Instant::now() >= self.expires
    }
    pub(super) fn expired(&mut self, cx: &mut Context<'_>) -> bool {
        Instant::now() >= self.expires || self.sleep.as_mut().poll(cx).is_ready()
    }
}
