mod bulk;
use super::PortalBulkBehaviour;
use crate::{
    PeerError,
    notification::NotifyLimits,
    notification_effects::{NotifyLane, NotifyLaneEvent},
    portal_config::PortalConfig,
    receipt_effects::{ReceiptLane, ReceiptLaneEvent, ReceiptRepo},
};
use futures::future::poll_fn;
use libp2p::{Multiaddr, Swarm, multiaddr::Protocol};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub enum PortalEvent {
    Ready { addresses: [Multiaddr; 3] },
    Receipt(ReceiptLaneEvent),
    Notification(NotifyLaneEvent),
    Ended,
}
pub struct PortalRound {
    pub events: [Option<PortalEvent>; 3],
}
pub struct PortalServer {
    repo: ReceiptRepo,
    receipt: ReceiptLane,
    notify: NotifyLane,
    bulk: Option<Swarm<PortalBulkBehaviour>>,
    #[cfg(test)]
    bulk_listener: libp2p::core::transport::ListenerId,
    until: Instant,
    wake: Pin<Box<tokio::time::Sleep>>,
    cursor: usize,
    listening: [Option<Multiaddr>; 3],
    ready: bool,
    closed: bool,
}
impl PortalServer {
    pub fn new(
        mut repo: ReceiptRepo,
        config: PortalConfig,
        duration: Duration,
    ) -> Result<Self, PeerError> {
        let addresses = super::configuration::server(&mut repo, &config, duration)?;
        let mut receipt = ReceiptLane::server(&repo)?;
        let mut notify = NotifyLane::server(&repo, NotifyLimits::default())?;
        let mut bulk = repo.transport().build_portal_bulk_lane()?;
        let until = Instant::now()
            .checked_add(duration)
            .ok_or(PeerError::Limit)?;
        receipt.listen(addresses[0].clone())?;
        let _bulk_listener = bulk
            .listen_on(addresses[1].clone())
            .map_err(|_| PeerError::Offline)?;
        notify.listen(addresses[2].clone())?;
        Ok(Self {
            repo,
            receipt,
            notify,
            bulk: Some(bulk),
            #[cfg(test)]
            bulk_listener: _bulk_listener,
            until,
            wake: Box::pin(tokio::time::sleep(Duration::from_millis(25))),
            cursor: 0,
            listening: std::array::from_fn(|_| None),
            ready: false,
            closed: false,
        })
    }
    pub async fn next_round(&mut self) -> Result<PortalRound, PeerError> {
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
    fn listen_observation(&mut self, lane: usize, mut address: Multiaddr) -> Result<(), PeerError> {
        let mut parts = address.iter();
        let (Some(Protocol::Ip4(ip)), Some(Protocol::Tcp(port))) = (parts.next(), parts.next())
        else {
            return Err(PeerError::Malformed);
        };
        if !ip.is_loopback() || port == 0 || parts.next().is_some() {
            return Err(PeerError::Unauthorized);
        }
        let peer = self.repo.transport().peer_id();
        if peer.to_bytes() != self.repo.local_public().peer {
            return Err(PeerError::Unauthorized);
        }
        address.push(Protocol::P2p(peer));
        if self
            .listening
            .iter()
            .enumerate()
            .any(|(i, existing)| i != lane && existing.as_ref() == Some(&address))
        {
            return Err(PeerError::Malformed);
        }
        self.listening[lane] = Some(address);
        Ok(())
    }
    fn poll_round(&mut self, cx: &mut Context<'_>) -> Poll<Result<PortalRound, PeerError>> {
        let result = self.poll_inner(cx);
        if matches!(result, Poll::Ready(Err(_))) {
            self.shutdown();
        }
        result
    }
    fn poll_inner(&mut self, cx: &mut Context<'_>) -> Poll<Result<PortalRound, PeerError>> {
        if !self.sweep()? {
            return Poll::Ready(Ok(PortalRound {
                events: [Some(PortalEvent::Ended), None, None],
            }));
        }
        let mut events: [Option<PortalEvent>; 3] = std::array::from_fn(|_| None);
        let mut count = 0;
        for offset in 0..3 {
            let lane = (self.cursor + offset) % 3;
            let event = match lane {
                0 => match self.receipt.poll(cx, &mut self.repo) {
                    Poll::Ready(Ok(Some(ReceiptLaneEvent::Listening(a)))) => {
                        self.listen_observation(0, a)?;
                        None
                    }
                    Poll::Ready(Ok(Some(event))) => Some(PortalEvent::Receipt(event)),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    _ => None,
                },
                1 => self.poll_bulk(cx)?,
                _ => match self.notify.poll(cx, &mut self.repo) {
                    Poll::Ready(Ok(Some(NotifyLaneEvent::Listening(a)))) => {
                        self.listen_observation(2, a)?;
                        None
                    }
                    Poll::Ready(Ok(Some(event))) => Some(PortalEvent::Notification(event)),
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    _ => None,
                },
            };
            if let Some(event) = event {
                events[count] = Some(event);
                count += 1;
            }
            if !self.sweep()? {
                return Poll::Ready(Ok(PortalRound {
                    events: [Some(PortalEvent::Ended), None, None],
                }));
            }
        }
        self.cursor = (self.cursor + 1) % 3;
        if !self.ready && self.listening.iter().all(Option::is_some) && count < 3 {
            events[count] = Some(PortalEvent::Ready {
                addresses: std::array::from_fn(|i| {
                    self.listening[i]
                        .as_ref()
                        .expect("checked complete")
                        .clone()
                }),
            });
            self.ready = true;
            count += 1;
        }
        if count > 0 {
            return Poll::Ready(Ok(PortalRound { events }));
        }
        if self.wake.as_mut().poll(cx).is_ready() {
            self.wake = Box::pin(tokio::time::sleep(Duration::from_millis(25)));
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }
    pub fn shutdown(&mut self) {
        self.closed = true;
        self.receipt.shutdown();
        self.notify.shutdown();
        self.bulk = None;
    }
}

#[cfg(test)]
#[path = "../../../tests/portal_internal/lifecycle.rs"]
mod lifecycle_test;
