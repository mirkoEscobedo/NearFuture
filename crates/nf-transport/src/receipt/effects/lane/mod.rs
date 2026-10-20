//! One foreground receipt Swarm. Protocol states and backend custody stay private.
use super::*;
use crate::{
    PeerError,
    receipt::{OriginalReceipt, SourceMinima},
};
use futures::Stream;
use libp2p::{Multiaddr, PeerId, Swarm, core::transport::ListenerId, swarm::ConnectionId};
use std::task::{Context, Poll};
#[cfg(test)]
pub(super) mod activation_test;
mod deadline;
mod events;
mod policy;
#[derive(Debug)]
pub enum ReceiptLaneEvent {
    Listening(Multiaddr),
    Authenticated {
        peer: PeerId,
    },
    Accepted {
        outcome: AdmittedReceipt,
        completion: Option<Box<ReceiptCompletion>>,
    },
    Closed {
        peer: PeerId,
    },
}
enum Endpoint {
    Server {
        handshake: Box<ReceiptServerHandshake>,
        operation: Option<Box<ReceiptOperationServer>>,
    },
    Client {
        handshake: Box<ReceiptClientHandshake>,
        operation: Option<Box<ReceiptOperationClient>>,
        slot: u8,
        original: Box<OriginalReceipt>,
        minimum: SourceMinima,
    },
}
/// The effect proof transitions are internal; callers use this foreground owner.
/// ```compile_fail
/// use nf_transport::receipt_effects::ReceiptClientHandshake;
/// let _ = ReceiptClientHandshake::new();
/// ```
pub struct ReceiptLane {
    swarm: Option<Swarm<ReceiptBehaviour>>,
    endpoint: Endpoint,
    binding: ReceiptConfig,
    local: nf_identity::model::PublicIdentity,
    connection: Option<(PeerId, ConnectionId)>,
    active: Option<policy::ActivePolicy>,
    query: Option<ReceiptQueryBinding>,
    listener: Option<ListenerId>,
    dialing: bool,
    closed: bool,
    deadline: Option<deadline::Deadline>,
}
impl ReceiptLane {
    pub fn server(repo: &ReceiptRepo) -> Result<Self, PeerError> {
        let local = repo.local_public();
        let pin = repo.config().server_pin;
        if local.account != pin.account || local.device != pin.device {
            return Err(PeerError::Unauthorized);
        }
        Self::new(
            repo,
            Endpoint::Server {
                handshake: Box::new(ReceiptServerHandshake::new()),
                operation: None,
            },
        )
    }
    pub fn client(repo: &ReceiptRepo, slot: u8) -> Result<Self, PeerError> {
        let (original, minimum) = repo.original_for_slot(slot)?;
        repo.validate_original(&original)?;
        Self::new(
            repo,
            Endpoint::Client {
                handshake: Box::new(ReceiptClientHandshake::new()),
                operation: None,
                slot,
                original: Box::new(original),
                minimum,
            },
        )
    }
    fn new(repo: &ReceiptRepo, endpoint: Endpoint) -> Result<Self, PeerError> {
        Ok(Self {
            swarm: Some(repo.build_receipt_lane()?),
            endpoint,
            binding: repo.config(),
            local: repo.local_public().clone(),
            connection: None,
            active: None,
            query: None,
            listener: None,
            dialing: false,
            closed: false,
            deadline: None,
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
        self.start_deadline();
        self.dialing = true;
        Ok(())
    }
    /// Queries only the immutable anchored original selected at construction.
    pub fn request_receipt(&mut self, repo: &mut ReceiptRepo) -> Result<(), PeerError> {
        self.request_inner(repo, None)
    }
    pub(crate) fn request_for_query(
        &mut self,
        repo: &mut ReceiptRepo,
        binding: ReceiptQueryBinding,
    ) -> Result<(), PeerError> {
        self.request_inner(repo, Some(binding))
    }
    fn request_inner(
        &mut self,
        repo: &mut ReceiptRepo,
        binding: Option<ReceiptQueryBinding>,
    ) -> Result<(), PeerError> {
        if let Some(binding) = &binding {
            binding.live()?;
        }
        if let Err(error) = self.guard(repo) {
            self.shutdown();
            return Err(error);
        }
        let (peer, id) = self.connection.ok_or(PeerError::Offline)?;
        if let Some(binding) = &binding {
            binding.live()?;
            let Endpoint::Client { slot, original, .. } = &self.endpoint else {
                return Err(PeerError::Session);
            };
            if binding.slot != *slot
                || !super::completion::same_original(&binding.original, original)
            {
                return Err(PeerError::Unauthorized);
            }
            repo.require_context(&binding.context)?;
            if !binding.minimum.admits(repo.protected_minimum(original)?) {
                return Err(PeerError::Policy);
            }
            binding.live()?;
        }
        let Endpoint::Client {
            operation: Some(operation),
            ..
        } = &mut self.endpoint
        else {
            return Err(PeerError::Session);
        };
        let begin = match &binding {
            Some(binding) => operation.begin_for_query(repo, binding.minimum, binding.not_after),
            None => operation.begin(repo),
        };
        let record = match begin {
            Ok(record) => record,
            Err(PeerError::Backpressure) => return Err(PeerError::Backpressure),
            Err(error) => {
                self.shutdown();
                return Err(error);
            }
        };
        if let Some(binding) = &binding {
            if let Err(error) = binding.live() {
                self.shutdown();
                return Err(error);
            }
            let crate::receipt::ReceiptBody::Begin { minimum, .. } = &record.body else {
                return Err(PeerError::Session);
            };
            if !minimum.admits(binding.minimum) {
                self.shutdown();
                return Err(PeerError::Policy);
            }
        }
        self.query = binding; // Reserve correlation before actual backend enqueue.
        self.deadline = Some(deadline::Deadline::new());
        if let Err(error) = operation.queue_request(
            record,
            peer,
            id,
            self.swarm.as_mut().ok_or(PeerError::Offline)?,
            repo,
        ) {
            self.shutdown();
            return Err(error);
        }
        Ok(())
    }
    /// Exactly one backend event per call; no borrowed repository state crosses an await.
    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
        repo: &mut ReceiptRepo,
    ) -> Poll<Result<Option<ReceiptLaneEvent>, PeerError>> {
        if self.closed {
            return Poll::Ready(Err(PeerError::Offline));
        }
        if self
            .deadline
            .as_mut()
            .is_some_and(|deadline| deadline.expired(cx))
        {
            self.shutdown();
            return Poll::Ready(Err(PeerError::Replay));
        }
        if let Err(error) = self.guard(repo) {
            self.shutdown();
            return Poll::Ready(Err(error));
        }
        // A synchronous policy reload may outlive the wakeup driver; refuse after it returns.
        if self
            .deadline
            .as_mut()
            .is_some_and(|deadline| deadline.expired(cx))
        {
            self.shutdown();
            return Poll::Ready(Err(PeerError::Replay));
        }
        if let Some(query) = &self.query
            && let Err(error) = query.live()
        {
            self.shutdown();
            return Poll::Ready(Err(error));
        }
        let Some(swarm) = self.swarm.as_mut() else {
            return Poll::Ready(Err(PeerError::Offline));
        };
        match std::pin::Pin::new(swarm).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                self.shutdown();
                Poll::Ready(Err(PeerError::Offline))
            }
            Poll::Ready(Some(event)) => {
                let result = self.event(event, repo);
                if result.is_err() {
                    self.shutdown();
                }
                Poll::Ready(result)
            }
        }
    }
    /// Encoded custody only; this is not a total heap or RSS measurement.
    pub fn pending_encoded(&self) -> (usize, usize, usize) {
        let (a, b) = match &self.endpoint {
            Endpoint::Server {
                handshake,
                operation,
            } => (
                handshake.pending_delivery(),
                operation
                    .as_deref()
                    .map_or((0, 0, 0), ReceiptOperationServer::pending_delivery),
            ),
            Endpoint::Client {
                handshake,
                operation,
                ..
            } => (
                handshake.pending_delivery(),
                operation
                    .as_deref()
                    .map_or((0, 0, 0), ReceiptOperationClient::pending_delivery),
            ),
        };
        (a.0 + b.0, a.1 + b.1, a.2 + b.2)
    }
    pub fn shutdown(&mut self) {
        self.reset();
        self.connection = None;
        self.listener = None;
        // Dropping the sole backend cancels pending upgrades as well as established connections.
        self.swarm.take();
        self.dialing = false;
        self.closed = true;
    }
    fn start_deadline(&mut self) {
        self.deadline = Some(deadline::Deadline::new());
    }
    fn reset(&mut self) {
        self.active = None;
        self.query = None;
        self.deadline = None;
        match &mut self.endpoint {
            Endpoint::Server {
                handshake,
                operation,
            } => {
                handshake.invalidate();
                *operation = None;
            }
            Endpoint::Client {
                handshake,
                operation,
                ..
            } => {
                handshake.invalidate();
                *operation = None;
            }
        }
    }
}
