//! Foreground observation of retained receipt and notification lanes.
mod configuration;
mod preparation;
use crate::{
    PeerError,
    notification_effects::{NotifyLane, NotifyLaneEvent},
    portal_config::{PortalConfig, PortalMode},
    receipt_effects::{AdmittedReceipt, ReceiptLane, ReceiptLaneEvent, ReceiptRepo},
};
use futures::future::poll_fn;
use libp2p::PeerId;
pub use preparation::PreparedPortalClient;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Two-lane observations. These do not establish bulk application authentication.
pub enum WatchEvent {
    ReceiptAuthenticated { peer: PeerId },
    Subscribed,
    Status(AdmittedReceipt),
    Ended,
}
pub struct WatchRound {
    pub events: [Option<WatchEvent>; 2],
}
pub struct PortalClient {
    repo: ReceiptRepo,
    receipt: ReceiptLane,
    notify: NotifyLane,
    until: Instant,
    wake: Pin<Box<tokio::time::Sleep>>,
    cursor: usize,
    receipt_authenticated: bool,
    subscribed: bool,
    query_in_flight: bool,
    closed: bool,
}
impl PortalClient {
    /// Fully prepares existing protected state without dialing or starting a runtime.
    /// Addresses may be supplied later to consuming start; every supplied pin is checked.
    pub fn prepare(
        repo: ReceiptRepo,
        config: PortalConfig,
        slot: u8,
    ) -> Result<PreparedPortalClient, PeerError> {
        PreparedPortalClient::new(repo, config, slot)
    }
    /// Normal Watch requires explicit addresses and never initializes a missing book.
    pub fn new(
        repo: ReceiptRepo,
        config: PortalConfig,
        slot: u8,
        duration: Duration,
    ) -> Result<Self, PeerError> {
        configuration::duration(duration)?;
        let addresses = super::configuration::addresses(&config, PortalMode::Watch)?;
        Self::prepare(repo, config, slot)?.start(addresses, duration)
    }
    pub async fn next_round(&mut self) -> Result<WatchRound, PeerError> {
        poll_fn(|cx| self.poll_round(cx)).await
    }
    fn sweep(&mut self) -> Result<bool, PeerError> {
        if self.closed {
            return Err(PeerError::Offline);
        }
        if Instant::now() >= self.until {
            self.shutdown();
            return Ok(false);
        }
        self.repo.with_current_read(|_| Ok(()))?;
        if Instant::now() >= self.until {
            self.shutdown();
            return Ok(false);
        }
        Ok(true)
    }
    fn poll_round(&mut self, cx: &mut Context<'_>) -> Poll<Result<WatchRound, PeerError>> {
        let result = self.poll_inner(cx);
        if matches!(result, Poll::Ready(Err(_))) {
            self.shutdown();
        }
        result
    }
    fn ended() -> WatchRound {
        WatchRound {
            events: [Some(WatchEvent::Ended), None],
        }
    }
    fn poll_inner(&mut self, cx: &mut Context<'_>) -> Poll<Result<WatchRound, PeerError>> {
        if !self.sweep()? {
            return Poll::Ready(Ok(Self::ended()));
        }
        let mut events = std::array::from_fn(|_| None);
        let mut count = 0;
        for offset in 0..2 {
            let lane = (self.cursor + offset) % 2;
            let event = match lane {
                0 => match self.receipt.poll(cx, &mut self.repo) {
                    Poll::Ready(Ok(Some(ReceiptLaneEvent::Authenticated { peer }))) => {
                        self.receipt_authenticated = true;
                        Some(WatchEvent::ReceiptAuthenticated { peer })
                    }
                    Poll::Ready(Ok(Some(ReceiptLaneEvent::Closed { .. }))) => {
                        return Poll::Ready(Err(PeerError::Offline));
                    }
                    Poll::Ready(Ok(Some(ReceiptLaneEvent::Accepted {
                        outcome,
                        completion,
                    }))) => {
                        if !self.query_in_flight {
                            return Poll::Ready(Err(PeerError::Session));
                        }
                        self.query_in_flight = false;
                        let Some(completion) = completion else {
                            self.notify.receipt_failure();
                            return Poll::Ready(Err(PeerError::Unsupported));
                        };
                        self.notify
                            .accept_receipt_completion(*completion, &mut self.repo)?;
                        Some(WatchEvent::Status(outcome))
                    }
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    _ => None,
                },
                _ => match self.notify.poll(cx, &mut self.repo) {
                    Poll::Ready(Ok(Some(NotifyLaneEvent::Subscribed))) => {
                        self.subscribed = true;
                        Some(WatchEvent::Subscribed)
                    }
                    Poll::Ready(Ok(Some(NotifyLaneEvent::Closed))) => {
                        return Poll::Ready(Err(PeerError::Offline));
                    }
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    _ => None,
                },
            };
            if let Some(event) = event {
                events[count] = Some(event);
                count += 1;
            }
            if !self.sweep()? {
                return Poll::Ready(Ok(Self::ended()));
            }
        }
        self.cursor = (self.cursor + 1) % 2;
        // Actual startup can arrive in either order; dirty state stays inside NotifyLane.
        if self.receipt_authenticated
            && self.subscribed
            && !self.query_in_flight
            && self.notify.dirty()
        {
            self.notify
                .request_followup(&mut self.receipt, &mut self.repo)?;
            self.query_in_flight = true;
            cx.waker().wake_by_ref();
        }
        // Correlated admission/persistence and query preparation may perform synchronous work.
        // Recheck current SQL and the unchanged original owner deadline before publication.
        if !self.sweep()? {
            return Poll::Ready(Ok(Self::ended()));
        }
        if count > 0 {
            return Poll::Ready(Ok(WatchRound { events }));
        }
        if self.wake.as_mut().poll(cx).is_ready() {
            self.wake = Box::pin(tokio::time::sleep(Duration::from_millis(25)));
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }
    pub fn shutdown(&mut self) {
        self.closed = true;
        self.receipt_authenticated = false;
        self.subscribed = false;
        self.query_in_flight = false;
        self.notify.receipt_failure();
        self.receipt.shutdown();
        self.notify.shutdown();
    }
}
