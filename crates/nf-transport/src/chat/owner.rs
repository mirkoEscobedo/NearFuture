use super::{
    ChatFrame, MAX_FRAME_BYTES, Refusal, WireContext,
    network::{ChatBehaviour, ChatBehaviourEvent},
};
use crate::{PeerError, identity::TransportIdentity};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    request_response::{Event, InboundRequestId, Message},
    swarm::{ConnectionId, SwarmEvent},
};
use nf_identity::{
    model::{MembershipState, ProtectedOperation, Roles},
    private_storage::{LocalIdentity, PrivateVault},
};
use nf_store::chat::{
    Author, ChallengeRequest, ChatPolicy, ChatStore, ChatStoreError, LocalReceiptIssuer,
    ProofAttempt, SignedMessage, codec as store_codec,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
const MAX_EXCHANGES: usize = 64;
const MAX_RESPONSES: usize = 2;
const PROOF_AGE: Duration = Duration::from_secs(5);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatPeerPin {
    pub(super) scope: nf_identity::model::Scope,
    pub(super) receiver: Author,
    pub(super) peer: PeerId,
    pub(super) key: [u8; 32],
    pub(super) minimum_membership: u64,
}
impl ChatPeerPin {
    /// Trusted LOCAL supervisor selection; no wire enrollment or key replacement.
    pub fn from_current(current: &MembershipState, receiver: Author) -> Result<Self, PeerError> {
        nf_identity::codec::validate_state(current).map_err(|_| PeerError::Unauthorized)?;
        let account = current
            .accounts
            .get(&receiver.account)
            .ok_or(PeerError::Unauthorized)?;
        let device = current
            .devices
            .get(&receiver.device)
            .ok_or(PeerError::Unauthorized)?;
        if device.revoked
            || device.account != receiver.account
            || !account.roles.contains(Roles::PLAYER)
        {
            return Err(PeerError::Unauthorized);
        }
        let peer = PeerId::from_bytes(&device.peer).map_err(|_| PeerError::Unauthorized)?;
        Ok(Self {
            scope: current.scope,
            receiver,
            peer,
            key: device.key,
            minimum_membership: current.revision,
        })
    }
}
pub(super) fn local_current(
    current: &MembershipState,
    local: &LocalIdentity,
    peer: PeerId,
) -> Result<(), PeerError> {
    let actor = Author {
        account: local.public.account,
        device: local.public.device,
    };
    let pin = ChatPeerPin::from_current(current, actor)?;
    let account = current
        .accounts
        .get(&actor.account)
        .ok_or(PeerError::Unauthorized)?;
    if pin.peer != peer
        || local.public.peer != peer.to_bytes()
        || pin.key != local.device_key.public_key()
        || pin.key != local.public.device_key
        || account.key != local.account_key.public_key()
        || account.key != local.public.account_key
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
pub(super) fn store_error(error: ChatStoreError) -> PeerError {
    match error {
        ChatStoreError::Limit => PeerError::Limit,
        ChatStoreError::Replay | ChatStoreError::Expired => PeerError::Replay,
        ChatStoreError::UnsupportedOperation => PeerError::Unsupported,
        ChatStoreError::Storage
        | ChatStoreError::Corrupt
        | ChatStoreError::Quarantined
        | ChatStoreError::MissingHistory
        | ChatStoreError::StaleBackup => PeerError::Storage,
        _ => PeerError::Unauthorized,
    }
}
fn refusal(error: PeerError) -> Refusal {
    match error {
        PeerError::Unsupported => Refusal::Unsupported,
        PeerError::Limit | PeerError::Backpressure => Refusal::Limit,
        PeerError::Replay => Refusal::Replay,
        PeerError::Offline => Refusal::Offline,
        _ => Refusal::Unauthorized,
    }
}
fn signed_digest(signed: &SignedMessage) -> Result<[u8; 32], PeerError> {
    let bytes = store_codec::encode_signed_message(signed).map_err(store_error)?;
    Ok(Sha256::digest(bytes).into())
}
struct Connection {
    peer: PeerId,
    id: ConnectionId,
    last: Instant,
    frames: super::admission::Window,
    account_hint: Option<nf_contract::identity::AccountId>,
    closing: bool,
}
struct Exchange {
    peer: PeerId,
    connection: ConnectionId,
    context: WireContext,
    digest: [u8; 32],
    challenge: [u8; 32],
    revision: u64,
    created: Instant,
}
struct HistoryExchange {
    peer: PeerId,
    connection: ConnectionId,
    context: WireContext,
    query: nf_store::chat::HistoryQuery,
    digest: [u8; 32],
    challenge: [u8; 32],
    revision: u64,
    created: Instant,
}
#[derive(Debug)]
pub enum ChatPeerServerEvent {
    Ready {
        address: Multiaddr,
        peer: PeerId,
    },
    Connected {
        peer: PeerId,
        connection: ConnectionId,
    },
    Disconnected {
        peer: PeerId,
        connection: ConnectionId,
    },
    Progress,
    Rejected(PeerError),
}
/// Foreground Chat owner. No service or background task; one independently bounded Noise lane.
pub struct ChatPeerServer {
    swarm: Swarm<ChatBehaviour>,
    store: ChatStore,
    local: LocalIdentity,
    peer: PeerId,
    policy: ChatPolicy,
    membership_revision: u64,
    connection: Option<Connection>,
    exchanges: BTreeMap<[u8; 16], Exchange>,
    history_exchanges: BTreeMap<[u8; 16], HistoryExchange>,
    responses: BTreeMap<(u8, InboundRequestId), (PeerId, ConnectionId)>,
    announced: bool,
    admission: super::admission::Admission,
}
impl ChatPeerServer {
    pub fn new(
        store: ChatStore,
        vault: &PrivateVault,
        policy: ChatPolicy,
    ) -> Result<Self, PeerError> {
        let identity = TransportIdentity::load(vault)?;
        let peer = identity.peer_id();
        let local = vault
            .load_identity(peer.to_bytes())
            .map_err(|_| PeerError::Storage)?;
        let current = store.current_membership().map_err(store_error)?;
        if current.scope != policy.scope {
            return Err(PeerError::Policy);
        }
        local_current(&current, &local, peer)?;
        let mut swarm = identity.build_chat()?;
        swarm
            .listen_on(
                "/ip4/127.0.0.1/tcp/0"
                    .parse()
                    .map_err(|_| PeerError::Offline)?,
            )
            .map_err(|_| PeerError::Offline)?;
        Ok(Self {
            swarm,
            store,
            local,
            peer,
            policy,
            membership_revision: current.revision,
            connection: None,
            exchanges: BTreeMap::new(),
            history_exchanges: BTreeMap::new(),
            responses: BTreeMap::new(),
            announced: false,
            admission: super::admission::Admission::new(Instant::now()),
        })
    }
    pub fn into_store(self) -> ChatStore {
        self.store
    }
    fn close(&mut self, peer: PeerId, id: ConnectionId) {
        if self
            .connection
            .as_ref()
            .is_some_and(|connection| connection.peer == peer && connection.id == id)
        {
            self.connection = None;
            self.exchanges.clear();
            self.history_exchanges.clear();
            self.responses.clear();
        }
        let _ = self.swarm.close_connection(id);
    }
    fn current(&mut self) -> Result<MembershipState, PeerError> {
        let current = self.store.current_membership().map_err(store_error)?;
        if current.scope != self.policy.scope {
            return Err(PeerError::Policy);
        }
        if current.revision != self.membership_revision {
            self.exchanges.clear();
            self.history_exchanges.clear();
            self.membership_revision = current.revision;
        }
        local_current(&current, &self.local, self.peer)?;
        Ok(current)
    }
    fn admit_frame(
        &mut self,
        peer: PeerId,
        id: ConnectionId,
        challenge: bool,
    ) -> Result<(), PeerError> {
        let now = Instant::now();
        let owned = self
            .connection
            .as_mut()
            .filter(|owned| owned.peer == peer && owned.id == id && !owned.closing)
            .ok_or(PeerError::Session)?;
        if let Err(error) = self.admission.frame(&mut owned.frames, now) {
            owned.closing = true;
            return Err(error);
        }
        // A prior binding may deny cheaply; it never admits or replaces fresh current authority.
        if challenge
            && owned
                .account_hint
                .is_some_and(|account| self.admission.blocked(account, now))
        {
            return Err(PeerError::Limit);
        }
        owned.last = now;
        Ok(())
    }
    fn chargeable_peer(
        &mut self,
        current: &MembershipState,
        peer: PeerId,
    ) -> Result<nf_contract::identity::AccountId, PeerError> {
        match super::admission::peer_account(current, peer) {
            Ok(account) => {
                if let Some(owned) = self.connection.as_mut() {
                    owned.account_hint = Some(account);
                }
                Ok(account)
            }
            Err(error) => {
                if let Some(owned) = self.connection.as_mut() {
                    owned.closing = true;
                }
                Err(error)
            }
        }
    }
    fn admit_challenge(
        &mut self,
        current: &MembershipState,
        peer: PeerId,
        account: nf_contract::identity::AccountId,
        actor: Author,
    ) -> Result<(), PeerError> {
        if let Err(error) = super::admission::actor_bound(current, peer, account, actor) {
            if let Some(owned) = self.connection.as_mut() {
                owned.closing = true;
            }
            return Err(error);
        }
        // Failed/expired challenges still spend capacity; no refund or retry amplification.
        self.admission.challenge(account, Instant::now())
    }
    fn response_finished(
        &mut self,
        protocol: u8,
        request: InboundRequestId,
        peer: PeerId,
        id: ConnectionId,
    ) {
        if self.responses.remove(&(protocol, request)) != Some((peer, id))
            || self
                .connection
                .as_ref()
                .is_some_and(|owned| owned.peer == peer && owned.id == id && owned.closing)
        {
            self.close(peer, id);
        }
    }
    fn prune_exchanges(&mut self) {
        self.exchanges
            .retain(|_, entry| entry.created.elapsed() < PROOF_AGE);
        self.history_exchanges
            .retain(|_, entry| entry.created.elapsed() < PROOF_AGE);
    }
    fn route(
        &mut self,
        frame: ChatFrame,
        peer: PeerId,
        connection: ConnectionId,
    ) -> Result<ChatFrame, PeerError> {
        let current = self.current()?;
        let account = self.chargeable_peer(&current, peer)?;
        match frame {
            ChatFrame::PostChallenge { context, signed } => {
                if context.policy_digest != store_codec::policy_digest(&self.policy) {
                    return Err(PeerError::Policy);
                }
                self.admit_challenge(&current, peer, account, signed.message.author)?;
                self.prune_exchanges();
                if self.exchanges.len() + self.history_exchanges.len() >= MAX_EXCHANGES {
                    return Err(PeerError::Limit);
                }
                let created = Instant::now();
                let issued = self
                    .store
                    .issue_challenge(
                        ChallengeRequest::Post {
                            request: context.request,
                            message: &signed,
                        },
                        &peer.to_bytes(),
                    )
                    .map_err(store_error)?;
                if issued.membership_revision != current.revision {
                    return Err(PeerError::Unauthorized);
                }
                let exchange = Exchange {
                    peer,
                    connection,
                    context,
                    digest: signed_digest(&signed)?,
                    challenge: issued.challenge,
                    revision: issued.membership_revision,
                    created,
                };
                if self.history_exchanges.contains_key(&issued.ticket)
                    || self.exchanges.insert(issued.ticket, exchange).is_some()
                {
                    return Err(PeerError::Replay);
                }
                Ok(ChatFrame::Issued {
                    context,
                    challenge: issued,
                })
            }
            ChatFrame::PostProof {
                context,
                signed,
                ticket,
                proof,
            } => {
                // Consume the adapter exchange exactly once, including malformed/stale attempts.
                self.history_exchanges.remove(&ticket);
                let exchange = self.exchanges.remove(&ticket).ok_or(PeerError::Replay)?;
                if exchange.peer != peer
                    || exchange.connection != connection
                    || exchange.context != context
                    || exchange.created.elapsed() >= PROOF_AGE
                    || exchange.digest != signed_digest(&signed)?
                    || exchange.revision != current.revision
                    || context.policy_digest != store_codec::policy_digest(&self.policy)
                {
                    return Err(PeerError::Unauthorized);
                }
                current
                    .authorize(
                        &proof,
                        &peer.to_bytes(),
                        &exchange.challenge,
                        exchange.revision,
                        ProtectedOperation::Chat,
                    )
                    .map_err(|_| PeerError::Unauthorized)?;
                // Consume the real store ticket and commit the original once through its existing authority.
                let observed_peer = peer.to_bytes();
                let posted = self
                    .store
                    .post(
                        context.request,
                        &signed,
                        ProofAttempt {
                            ticket,
                            proof: &proof,
                            peer: &observed_peer,
                        },
                    )
                    .map_err(store_error)?;
                // An idempotent retry may name an alias; the store-returned first original remains receipt authority.
                let receiver_peer = self.peer.to_bytes();
                let receipt = self
                    .store
                    .issue_delivery_receipt(
                        posted.original_request,
                        LocalReceiptIssuer {
                            policy: self.policy,
                            receiver: Author {
                                account: self.local.public.account,
                                device: self.local.public.device,
                            },
                            peer: &receiver_peer,
                            device_key: &self.local.device_key,
                        },
                    )
                    .map_err(store_error)?;
                Ok(ChatFrame::Delivered {
                    context,
                    signed: Box::new(receipt),
                })
            }
            _ => Err(PeerError::Unsupported),
        }
    }
    fn history_route(
        &mut self,
        frame: super::history::HistoryFrame,
        peer: PeerId,
        connection: ConnectionId,
    ) -> Result<super::history::HistoryFrame, PeerError> {
        use super::history::{HistoryFrame, binding, validate_query};
        let current = self.current()?;
        let account = self.chargeable_peer(&current, peer)?;
        let (context, query) = frame.context_query();
        validate_query(context, query)?;
        if context.policy_digest != store_codec::policy_digest(&self.policy) {
            return Err(PeerError::Policy);
        }
        match frame {
            HistoryFrame::Challenge { .. } => {
                self.admit_challenge(&current, peer, account, query.reader)?;
                self.prune_exchanges();
                if self.exchanges.len() + self.history_exchanges.len() >= MAX_EXCHANGES {
                    return Err(PeerError::Limit);
                }
                let created = Instant::now();
                let issued = self
                    .store
                    .issue_challenge(ChallengeRequest::History(&query), &peer.to_bytes())
                    .map_err(store_error)?;
                if issued.membership_revision != current.revision {
                    return Err(PeerError::Unauthorized);
                }
                let exchange = HistoryExchange {
                    peer,
                    connection,
                    context,
                    query,
                    digest: binding(context, query)?,
                    challenge: issued.challenge,
                    revision: issued.membership_revision,
                    created,
                };
                if self.exchanges.contains_key(&issued.ticket)
                    || self
                        .history_exchanges
                        .insert(issued.ticket, exchange)
                        .is_some()
                {
                    return Err(PeerError::Replay);
                }
                Ok(HistoryFrame::Issued {
                    context,
                    query,
                    challenge: issued,
                })
            }
            HistoryFrame::Proof { ticket, proof, .. } => {
                // Adapter admission is consumed once before checking every correlated physical/query field.
                self.exchanges.remove(&ticket);
                let exchange = self
                    .history_exchanges
                    .remove(&ticket)
                    .ok_or(PeerError::Replay)?;
                if exchange.peer != peer
                    || exchange.connection != connection
                    || exchange.context != context
                    || exchange.query != query
                    || exchange.digest != binding(context, query)?
                    || exchange.created.elapsed() >= PROOF_AGE
                    || exchange.revision != current.revision
                {
                    return Err(PeerError::Unauthorized);
                }
                current
                    .authorize(
                        &proof,
                        &peer.to_bytes(),
                        &exchange.challenge,
                        exchange.revision,
                        ProtectedOperation::Chat,
                    )
                    .map_err(|_| PeerError::Unauthorized)?;
                // The real store transaction consumes this exact fresh reader ticket and returns permitted source history.
                let observed_peer = peer.to_bytes();
                let page = self
                    .store
                    .history(
                        &query,
                        ProofAttempt {
                            ticket,
                            proof: &proof,
                            peer: &observed_peer,
                        },
                    )
                    .map_err(store_error)?;
                super::history::validate_page(query, &page)?;
                Ok(HistoryFrame::Page {
                    context,
                    query,
                    page: Box::new(page),
                })
            }
            _ => Err(PeerError::Unsupported),
        }
    }
    fn history_request(
        &mut self,
        peer: PeerId,
        connection: ConnectionId,
        id: InboundRequestId,
        frame: super::history::HistoryFrame,
        channel: libp2p::request_response::ResponseChannel<super::history::HistoryFrame>,
    ) -> Result<ChatPeerServerEvent, PeerError> {
        use super::history::HistoryFrame;
        self.connection
            .as_ref()
            .filter(|owned| owned.peer == peer && owned.id == connection && !owned.closing)
            .ok_or(PeerError::Session)?;
        // One shared two-slot4096 budget covers both protocols before issuing tickets or reading history.
        if self.responses.len() >= MAX_RESPONSES || self.responses.contains_key(&(2, id)) {
            return Err(PeerError::Backpressure);
        }
        self.responses.insert((2, id), (peer, connection));
        if !frame.is_request() {
            return Err(PeerError::Malformed);
        }
        let (context, query) = frame.context_query();
        let is_challenge = matches!(&frame, HistoryFrame::Challenge { .. });
        let (response, rejected) = match self
            .admit_frame(peer, connection, is_challenge)
            .and_then(|()| self.history_route(frame, peer, connection))
        {
            Ok(response) => (response, None),
            Err(PeerError::Storage) => return Err(PeerError::Storage),
            Err(error) => (
                HistoryFrame::Refused {
                    context,
                    query,
                    reason: refusal(error),
                },
                Some(error),
            ),
        };
        if super::history::encode(&response)?.len() > MAX_FRAME_BYTES {
            return Err(PeerError::Limit);
        }
        self.swarm
            .behaviour_mut()
            .history
            .send_response(channel, response)
            .map_err(|_| PeerError::Offline)?;
        Ok(rejected.map_or(ChatPeerServerEvent::Progress, ChatPeerServerEvent::Rejected))
    }
    fn request(
        &mut self,
        peer: PeerId,
        connection: ConnectionId,
        id: InboundRequestId,
        frame: ChatFrame,
        channel: libp2p::request_response::ResponseChannel<ChatFrame>,
    ) -> Result<ChatPeerServerEvent, PeerError> {
        self.connection
            .as_ref()
            .filter(|owned| owned.peer == peer && owned.id == connection && !owned.closing)
            .ok_or(PeerError::Session)?;
        // Reserve a worst-case4096-byte response slot before issuing any ticket or later durable post.
        if self.responses.len() >= MAX_RESPONSES || self.responses.contains_key(&(1, id)) {
            return Err(PeerError::Backpressure);
        }
        self.responses.insert((1, id), (peer, connection));
        let context = match &frame {
            ChatFrame::PostChallenge { context, .. } | ChatFrame::PostProof { context, .. } => {
                *context
            }
            _ => return Err(PeerError::Malformed),
        };
        let is_challenge = matches!(&frame, ChatFrame::PostChallenge { .. });
        let (response, rejected) = match self
            .admit_frame(peer, connection, is_challenge)
            .and_then(|()| self.route(frame, peer, connection))
        {
            Ok(response) => (response, None),
            Err(PeerError::Storage) => return Err(PeerError::Storage),
            Err(error) => (
                ChatFrame::Refused {
                    context,
                    reason: refusal(error),
                },
                Some(error),
            ),
        };
        if super::codec::encode(&response)?.len() > MAX_FRAME_BYTES {
            return Err(PeerError::Limit);
        }
        self.swarm
            .behaviour_mut()
            .messages
            .send_response(channel, response)
            .map_err(|_| PeerError::Offline)?;
        Ok(rejected.map_or(ChatPeerServerEvent::Progress, ChatPeerServerEvent::Rejected))
    }
    pub async fn next(&mut self) -> Result<ChatPeerServerEvent, PeerError> {
        self.prune_exchanges();
        if let Some(connection) = &self.connection
            && connection.last.elapsed() >= Duration::from_secs(10)
        {
            let (peer, id) = (connection.peer, connection.id);
            self.close(peer, id);
        }
        let event = tokio::select! { event = self.swarm.select_next_some() => event,
        _ = tokio::time::sleep(Duration::from_millis(25)) => return Ok(ChatPeerServerEvent::Progress) };
        match event {
            SwarmEvent::NewListenAddr { address, .. } if !self.announced => {
                self.announced = true;
                Ok(ChatPeerServerEvent::Ready {
                    address: address.with(libp2p::multiaddr::Protocol::P2p(self.peer)),
                    peer: self.peer,
                })
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                ..
            } => {
                if self.connection.is_some() {
                    self.close(peer_id, connection_id);
                    return Ok(ChatPeerServerEvent::Rejected(PeerError::Backpressure));
                }
                self.connection = Some(Connection {
                    peer: peer_id,
                    id: connection_id,
                    last: Instant::now(),
                    frames: super::admission::Window::connection(Instant::now()),
                    account_hint: None,
                    closing: false,
                });
                Ok(ChatPeerServerEvent::Connected {
                    peer: peer_id,
                    connection: connection_id,
                })
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                connection_id,
                ..
            } => {
                self.close(peer_id, connection_id);
                Ok(ChatPeerServerEvent::Disconnected {
                    peer: peer_id,
                    connection: connection_id,
                })
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message {
                peer,
                connection_id,
                message:
                    Message::Request {
                        request_id,
                        request,
                        channel,
                    },
            })) => match self.request(peer, connection_id, request_id, request, channel) {
                Ok(event) => Ok(event),
                Err(error) => {
                    self.close(peer, connection_id);
                    if error == PeerError::Storage {
                        Err(error)
                    } else {
                        Ok(ChatPeerServerEvent::Rejected(error))
                    }
                }
            },
            SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::ResponseSent {
                peer,
                connection_id,
                request_id,
            })) => {
                self.response_finished(1, request_id, peer, connection_id);
                Ok(ChatPeerServerEvent::Progress)
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(
                Event::InboundFailure {
                    peer,
                    connection_id,
                    ..
                }
                | Event::OutboundFailure {
                    peer,
                    connection_id,
                    ..
                },
            )) => {
                self.close(peer, connection_id);
                Ok(ChatPeerServerEvent::Rejected(PeerError::Offline))
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::Message {
                peer,
                connection_id,
                message:
                    Message::Request {
                        request_id,
                        request,
                        channel,
                    },
            })) => match self.history_request(peer, connection_id, request_id, request, channel) {
                Ok(event) => Ok(event),
                Err(error) => {
                    self.close(peer, connection_id);
                    if error == PeerError::Storage {
                        Err(error)
                    } else {
                        Ok(ChatPeerServerEvent::Rejected(error))
                    }
                }
            },
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::ResponseSent {
                peer,
                connection_id,
                request_id,
            })) => {
                self.response_finished(2, request_id, peer, connection_id);
                Ok(ChatPeerServerEvent::Progress)
            }
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(
                Event::InboundFailure {
                    peer,
                    connection_id,
                    ..
                }
                | Event::OutboundFailure {
                    peer,
                    connection_id,
                    ..
                },
            )) => {
                self.close(peer, connection_id);
                Ok(ChatPeerServerEvent::Rejected(PeerError::Offline))
            }
            SwarmEvent::ListenerClosed { .. } | SwarmEvent::ListenerError { .. } => {
                Err(PeerError::Offline)
            }
            _ => Ok(ChatPeerServerEvent::Progress),
        }
    }
}
