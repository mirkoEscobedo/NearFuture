//! Cooperative monotonic expiry plus an asynchronous wakeup; no injected runtime clock.
use std::{
    future::Future,
    pin::Pin,
    task::Context,
    time::{Duration, Instant},
};
pub(super) struct Deadline {
    expires: Instant,
    wake: Pin<Box<tokio::time::Sleep>>,
}
impl Deadline {
    pub(super) fn new() -> Self {
        let window = Duration::from_secs(5);
        Self {
            expires: Instant::now() + window,
            wake: Box::pin(tokio::time::sleep(window)),
        }
    }
    pub(super) fn expired(&mut self, cx: &mut Context<'_>) -> bool {
        Instant::now() >= self.expires || self.wake.as_mut().poll(cx).is_ready()
    }
}
