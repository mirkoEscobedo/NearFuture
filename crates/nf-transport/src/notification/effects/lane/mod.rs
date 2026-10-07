//! One bounded foreground notification backend; all protocol and completion custody is private.
use super::*;
use crate::{
    PeerError,
    notification::{NotifyLimits, NotifyRecord},
    receipt::{OriginalReceipt, SourceMinima},
    receipt_effects::{ReceiptConfig, ReceiptRepo},
};
use futures::Stream;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    core::transport::ListenerId,
    request_response::{InboundRequestId, OutboundRequestId},
    swarm::ConnectionId,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
#[cfg(test)]
pub(in crate::notification_effects) mod completion_test;
mod deadline;
mod events;
mod guard;
mod query;
mod send;
use deadline::Deadline;
#[derive(Debug)]
pub enum NotifyLaneEvent {
    Listening(Multiaddr),
    Subscribed,
    Dirty,
    Closed,
}
enum Endpoint {
    Server {
        handshake: Option<NotifyServerHandshake>,
        session: Option<NotifySession>,
        broker: Option<NotifyBroker>,
    },
    Client {
        handshake: Option<NotifyClientHandshake>,
        subscriber: Option<NotifySubscriber>,
        original: OriginalReceipt,
        slot: u8,
        lifetime: u16,
    },
}
struct Outbound {
    id: OutboundRequestId,
    peer: PeerId,
    connection: ConnectionId,
    emission: NotifyEmission,
}
struct Inbound {
    id: InboundRequestId,
    peer: PeerId,
    connection: ConnectionId,
    emission: NotifyEmission,
}
pub struct NotifyLane {
    swarm: Option<Swarm<NotifyBehaviour>>,
    endpoint: Endpoint,
    policy: NotifyPolicy,
    binding: ReceiptConfig,
    local: nf_identity::model::PublicIdentity,
    connection: Option<(PeerId, ConnectionId)>,
    listener: Option<ListenerId>,
    dialing: bool,
    closed: bool,
    outbound: Option<Outbound>,
    inbound: Option<Inbound>,
    pending: Option<Deadline>,
    lifetime: Option<Deadline>,
    observe: Option<Pin<Box<tokio::time::Sleep>>>,
    query: Option<query::Query>,
}
impl NotifyLane {
    pub fn server(repo: &ReceiptRepo, limits: NotifyLimits) -> Result<Self, PeerError> {
        let c = repo.config();
        let local = repo.local_public();
        if c.server_pin.account != local.account || c.server_pin.device != local.device {
            return Err(PeerError::Unauthorized);
        }
        Self::new(
            repo,
            limits,
            Endpoint::Server {
                handshake: None,
                session: None,
                broker: None,
            },
        )
    }
    pub fn client(
        repo: &ReceiptRepo,
        slot: u8,
        lifetime: u16,
        limits: NotifyLimits,
    ) -> Result<Self, PeerError> {
        if !(1..=30).contains(&lifetime) {
            return Err(PeerError::Limit);
        }
        let (original, _) = repo.original_for_slot(slot)?;
        repo.selector(&original)?;
        Self::new(
            repo,
            limits,
            Endpoint::Client {
                handshake: None,
                subscriber: None,
                original,
                slot,
                lifetime,
            },
        )
    }
    fn new(
        repo: &ReceiptRepo,
        limits: NotifyLimits,
        endpoint: Endpoint,
    ) -> Result<Self, PeerError> {
        limits.validate()?;
        let c = repo.config();
        let swarm = repo.transport().build_notification_lane()?;
        crate::session::local_matches(repo.local_public(), *swarm.local_peer_id())?;
        Ok(Self {
            swarm: Some(swarm),
            endpoint,
            policy: NotifyPolicy {
                scope: c.scope,
                ruleset: c.ruleset,
                content: c.content,
                limits,
                minimum_membership: c.minimum_membership,
            },
            binding: c,
            local: repo.local_public().clone(),
            connection: None,
            listener: None,
            dialing: false,
            closed: false,
            outbound: None,
            inbound: None,
            pending: None,
            lifetime: None,
            observe: None,
            query: None,
        })
    }
    pub fn listen(&mut self, address: Multiaddr) -> Result<(), PeerError> {
        if self.closed || self.listener.is_some() || address.len() > 4096 {
            return Err(PeerError::Backpressure);
        }
        self.listener = Some(
            self.swarm
                .as_mut()
                .ok_or(PeerError::Offline)?
                .listen_on(address)
                .map_err(|_| PeerError::Offline)?,
        );
        Ok(())
    }
    pub fn dial(&mut self, address: Multiaddr) -> Result<(), PeerError> {
        if self.closed || self.dialing || self.connection.is_some() || address.len() > 4096 {
            return Err(PeerError::Backpressure);
        }
        self.swarm
            .as_mut()
            .ok_or(PeerError::Offline)?
            .dial(address)
            .map_err(|_| PeerError::Offline)?;
        self.dialing = true;
        self.start_pending();
        Ok(())
    }
    fn start_pending(&mut self) {
        self.pending = Some(Deadline::after(Duration::from_secs(5)));
    }
    /// Exactly one actual backend event per opportunity; no repository borrow survives an await.
    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
        repo: &mut ReceiptRepo,
    ) -> Poll<Result<Option<NotifyLaneEvent>, PeerError>> {
        self.poll_core(cx, repo, || {})
    }
    #[cfg(test)]
    pub(super) fn poll_after_read(
        &mut self,
        cx: &mut Context<'_>,
        repo: &mut ReceiptRepo,
        after_read: impl FnOnce(),
    ) -> Poll<Result<Option<NotifyLaneEvent>, PeerError>> {
        self.poll_core(cx, repo, after_read)
    }
    fn poll_core(
        &mut self,
        cx: &mut Context<'_>,
        repo: &mut ReceiptRepo,
        after_read: impl FnOnce(),
    ) -> Poll<Result<Option<NotifyLaneEvent>, PeerError>> {
        if self.closed {
            return Poll::Ready(Err(PeerError::Offline));
        }
        if self.expired(cx) {
            self.shutdown();
            return Poll::Ready(Err(PeerError::Replay));
        }
        if let Err(e) = self.guard(repo) {
            self.shutdown();
            return Poll::Ready(Err(e));
        }
        after_read();
        if self
            .observe
            .as_mut()
            .is_some_and(|d| std::future::Future::poll(d.as_mut(), cx).is_ready())
        {
            self.observe = Some(Box::pin(tokio::time::sleep(Duration::from_millis(100))));
            if let Err(e) = self.observe_retained(repo) {
                self.shutdown();
                return Poll::Ready(Err(e));
            }
        }
        if self.expired(cx) {
            self.shutdown();
            return Poll::Ready(Err(PeerError::Replay));
        }
        let result = match self.swarm.as_mut() {
            Some(s) => Pin::new(s).poll_next(cx),
            None => {
                return Poll::Ready(Err(PeerError::Offline));
            }
        };
        match result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                self.shutdown();
                Poll::Ready(Err(PeerError::Offline))
            }
            Poll::Ready(Some(e)) => {
                let result = self.event(e, repo);
                if result.is_err() {
                    self.shutdown();
                }
                if matches!(result, Ok(None)) {
                    cx.waker().wake_by_ref();
                }
                Poll::Ready(result)
            }
        }
    }
    fn expired(&mut self, cx: &mut Context<'_>) -> bool {
        [&mut self.pending, &mut self.lifetime]
            .into_iter()
            .any(|d| d.as_mut().is_some_and(|d| d.expired(cx)))
    }
    /// No incoming wire command invokes this local retained-projection observation.
    pub fn observe_retained(&mut self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
        self.guard(repo)?;
        if self.outbound.is_some() || self.inbound.is_some() {
            return Ok(());
        }
        let Some((peer, id)) = self.connection else {
            return Ok(());
        };
        if let Endpoint::Server {
            broker: Some(b), ..
        } = &mut self.endpoint
            && let Some(record) = b.next_notice(peer, id, repo)?
        {
            self.queue_request(record, peer, id, repo)?;
            self.start_pending();
        }
        Ok(())
    }
    pub fn pending_encoded(&self) -> (usize, usize, usize) {
        let records = [
            self.outbound.as_ref().map(|r| r.emission.record()),
            self.inbound.as_ref().map(|r| r.emission.record()),
        ];
        let mut count = 0;
        let mut body = 0;
        for r in records.into_iter().flatten() {
            if let Ok(b) = crate::notification::encode_body(
                r,
                crate::notification::PROTOCOL,
                self.policy.limits,
            ) {
                count += 1;
                body += b.len();
            }
        }
        (count, body, count * 4)
    }
    pub fn dirty(&self) -> bool {
        matches!(&self.endpoint,Endpoint::Client{subscriber:Some(s),..} if s.dirty())
    }
    pub fn shutdown(&mut self) {
        self.closed = true;
        self.swarm.take();
        self.connection = None;
        self.listener = None;
        self.dialing = false;
        self.outbound = None;
        self.inbound = None;
        self.pending = None;
        self.lifetime = None;
        self.observe = None;
        self.query = None;
        match &mut self.endpoint {
            Endpoint::Server {
                handshake,
                session,
                broker,
            } => {
                if let Some(h) = handshake {
                    h.invalidate();
                }
                if let Some(s) = session {
                    s.invalidate();
                }
                if let Some(b) = broker {
                    b.invalidate();
                }
            }
            Endpoint::Client {
                handshake,
                subscriber,
                ..
            } => {
                if let Some(h) = handshake {
                    h.invalidate();
                }
                if let Some(s) = subscriber {
                    s.invalidate();
                }
            }
        }
    }
}
