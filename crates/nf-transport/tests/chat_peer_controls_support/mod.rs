use crate::original_fixture::{Fixture, author};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    request_response::{Event, Message},
    swarm::{ConnectionId, SwarmEvent},
};
use nf_contract::identity::RequestId;
use nf_identity::{
    model::{DeviceProof, DeviceRevocation, ProtectedOperation},
    private_storage::{LocalIdentity, PrivateVault},
    rotation::revocation_digest,
    signing::device_digest,
};
use nf_store::chat::{
    ChallengeRequest, ChatStore, HistoryEntry, HistoryPage, HistoryQuery, IssuedChallenge,
    KnownChatFrontiers, SignedMessage, codec,
    outbox::{ClientOutbox, OutgoingEntry, OutgoingState},
};
use nf_transport::{
    PeerError,
    chat::{
        ChatFrame, ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, Refusal, WireContext,
        deliver_pending,
        network::{ChatBehaviour, ChatBehaviourEvent},
    },
    identity::TransportIdentity,
};
use sha2::{Digest, Sha256};
use std::fs;
pub struct Prepared {
    pub receiver: Option<ChatStore>,
    pub outbox: ClientOutbox,
    pending: OutgoingEntry,
    receiver_before: Vec<u8>,
    outbox_before: Vec<u8>,
    membership: u64,
}
impl Prepared {
    pub fn new(f: &Fixture, revoked: bool) -> Self {
        let mut receiver = ChatStore::create(&f.receiver_path, &f.policy, &f.state).unwrap();
        let membership = if revoked {
            let change = DeviceRevocation {
                scope: f.policy.scope,
                issuer: f.alice.public.account,
                device: f.alice.public.device,
                frontier: 1,
            };
            let signature = f.alice.account_key.sign(&revocation_digest(&change));
            let next = receiver.revoke_device(&change, &signature).unwrap();
            assert_eq!(next.revision, 2);
            assert!(next.devices[&f.alice.public.device].revoked);
            2
        } else {
            1
        };
        let mut outbox = ClientOutbox::create(&f.outbox_path, &f.profile).unwrap();
        let mut pending = f.expected();
        pending.state = OutgoingState::Pending;
        assert_eq!(outbox.enqueue(f.request(), &f.signed), Ok(pending.clone()));
        drop(outbox);
        drop(receiver);
        let receiver_before = fs::read(&f.receiver_path).unwrap();
        let outbox_before = fs::read(&f.outbox_path).unwrap();
        let receiver = ChatStore::open_existing(
            &f.receiver_path,
            &f.policy,
            KnownChatFrontiers {
                scope: f.policy.scope,
                revision: 0,
                membership_revision: membership,
            },
        )
        .unwrap();
        let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
        Self {
            receiver: Some(receiver),
            outbox,
            pending,
            receiver_before,
            outbox_before,
            membership,
        }
    }
    pub fn server(&mut self, f: &Fixture) -> ChatPeerServer {
        ChatPeerServer::new(self.receiver.take().unwrap(), &f.server_vault, f.policy).unwrap()
    }
    /// Owners close before byte snapshots/reopen. History oracle uses genuine current Bob reader proof.
    pub fn finish(self, mut receiver: ChatStore, f: &Fixture, revision: u64) {
        let known = KnownChatFrontiers {
            scope: f.policy.scope,
            revision,
            membership_revision: self.membership,
        };
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(self.outbox.known_revision(), Ok(1));
        assert_eq!(self.outbox.entry([81; 16]), Ok(Some(self.pending.clone())));
        let expected_page = HistoryPage {
            entries: if revision == 0 {
                vec![]
            } else {
                vec![HistoryEntry {
                    receiver_cursor: 1,
                    signed: f.signed.clone(),
                }]
            },
            next_cursor: revision,
        };
        assert_eq!(history(&mut receiver, f), expected_page);
        drop(receiver);
        drop(self.outbox);
        let receiver_closed = fs::read(&f.receiver_path).unwrap();
        let outbox_closed = fs::read(&f.outbox_path).unwrap();
        if revision == 0 {
            assert_eq!(receiver_closed, self.receiver_before);
        }
        assert_eq!(outbox_closed, self.outbox_before);
        let mut receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(history(&mut receiver, f), expected_page);
        assert_eq!(outbox.known_revision(), Ok(1));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(self.pending)));
        drop(receiver);
        drop(outbox);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), receiver_closed);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_closed);
    }
}
fn history(receiver: &mut ChatStore, f: &Fixture) -> HistoryPage {
    let query = HistoryQuery {
        request: RequestId::from_bytes([110; 16]),
        reader: author(&f.bob),
        channel: nf_store::chat::Channel::General,
        after_cursor: 0,
        limit: 2,
    };
    let issued = receiver
        .issue_challenge(ChallengeRequest::History(&query), &f.bob.public.peer)
        .unwrap();
    let proof = proof(&f.bob, f.policy.scope, issued);
    receiver
        .history(
            &query,
            nf_store::chat::ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &f.bob.public.peer,
            },
        )
        .unwrap()
}
pub fn context(f: &Fixture) -> WireContext {
    WireContext {
        policy_digest: codec::policy_digest(&f.policy),
        request: f.request(),
    }
}
pub fn proof(
    local: &LocalIdentity,
    scope: nf_identity::model::Scope,
    issued: IssuedChallenge,
) -> DeviceProof {
    let mut proof = DeviceProof {
        scope,
        account: local.public.account,
        device: local.public.device,
        peer: local.public.peer.clone(),
        challenge: issued.challenge,
        frontier: issued.membership_revision,
        signature: [0; 64],
    };
    proof.signature = local.device_key.sign(&device_digest(&proof).unwrap());
    proof
}
pub async fn ready(server: &mut ChatPeerServer, f: &Fixture) -> Multiaddr {
    loop {
        if let ChatPeerServerEvent::Ready { address, peer } = server.next().await.unwrap() {
            assert_eq!(peer, f.server_peer);
            return address;
        }
    }
}
pub async fn deliver(
    server: &mut ChatPeerServer,
    outbox: &mut ClientOutbox,
    f: &Fixture,
    address: Multiaddr,
) -> Result<OutgoingEntry, PeerError> {
    let pin = ChatPeerPin::from_current(&f.state, author(&f.bob)).unwrap();
    let delivery = deliver_pending(outbox, &f.client_vault, &f.state, &pin, address, [81; 16]);
    tokio::pin!(delivery);
    loop {
        tokio::select! { result = &mut delivery => return result,
        event = server.next() => { event.unwrap(); } }
    }
}
pub struct RawPeer {
    pub swarm: Swarm<ChatBehaviour>,
    pub peer: PeerId,
    connection: Option<(ConnectionId, ConnectionId)>,
}
impl RawPeer {
    pub fn new(vault: &PrivateVault) -> Self {
        let identity = TransportIdentity::load(vault).unwrap();
        Self {
            peer: identity.peer_id(),
            swarm: identity.build_chat().unwrap(),
            connection: None,
        }
    }
    pub async fn connect(
        &mut self,
        server: &mut ChatPeerServer,
        address: Multiaddr,
        expected: PeerId,
    ) -> (ConnectionId, ConnectionId) {
        self.swarm.dial(address).unwrap();
        let mut client = None;
        let mut owner = None;
        while client.is_none() || owner.is_none() {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::ConnectionEstablished { peer_id, connection_id, .. } => {
                        assert_eq!(peer_id, expected); client = Some(connection_id);
                    }
                    SwarmEvent::OutgoingConnectionError { error, .. } => panic!("unexpected genuine connect failure: {error}"),
                    _ => {}
                },
                event = server.next() => if let ChatPeerServerEvent::Connected { peer, connection } = event.unwrap() {
                    assert_eq!(peer, self.peer); owner = Some(connection);
                }
            }
        }
        let ids = (client.unwrap(), owner.unwrap());
        self.connection = Some(ids);
        ids
    }
    pub async fn exchange(
        &mut self,
        server: &mut ChatPeerServer,
        expected: PeerId,
        frame: ChatFrame,
    ) -> ChatFrame {
        let id = self
            .swarm
            .behaviour_mut()
            .messages
            .send_request(&expected, frame);
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message { peer, connection_id,
                        message: Message::Response { request_id, response } })) => {
                        assert_eq!(peer, expected); assert_eq!(Some(connection_id), self.connection.map(|ids| ids.0));
                        assert_eq!(request_id, id); return response;
                    }
                    SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::OutboundFailure { error, .. })) => panic!("unexpected exchange failure: {error}"),
                    _ => {}
                },
                event = server.next() => { event.unwrap(); }
            }
        }
    }
    pub async fn disconnect(&mut self, server: &mut ChatPeerServer, expected: PeerId) {
        let (client_id, owner_id) = self.connection.take().unwrap();
        assert!(self.swarm.close_connection(client_id));
        let mut client_closed = false;
        let mut owner_closed = false;
        while !client_closed || !owner_closed {
            tokio::select! {
                event = self.swarm.select_next_some() => if let SwarmEvent::ConnectionClosed { peer_id, connection_id, .. } = event {
                    assert_eq!(peer_id, expected); assert_eq!(connection_id, client_id); client_closed = true;
                },
                event = server.next() => if let ChatPeerServerEvent::Disconnected { peer, connection } = event.unwrap() {
                    assert_eq!(peer, self.peer); assert_eq!(connection, owner_id); owner_closed = true;
                }
            }
        }
    }
    pub async fn rejected_second(
        &mut self,
        server: &mut ChatPeerServer,
        first: &mut RawPeer,
        address: Multiaddr,
    ) {
        self.swarm.dial(address).unwrap();
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::OutgoingConnectionError { .. } => return,
                    SwarmEvent::ConnectionClosed { .. } => return, // Transient client Noise establishment grants no foreground-owner admission.
                    _ => {}
                },
                event = first.swarm.select_next_some() => if let SwarmEvent::ConnectionClosed { .. } = event { panic!("first session lost under second-connection pressure"); },
                event = server.next() => if let ChatPeerServerEvent::Connected { .. } = event.unwrap() { panic!("second foreground connection unexpectedly admitted"); }
            }
        }
    }
}
pub async fn issued(
    raw: &mut RawPeer,
    server: &mut ChatPeerServer,
    f: &Fixture,
) -> IssuedChallenge {
    match raw
        .exchange(
            server,
            f.server_peer,
            ChatFrame::PostChallenge {
                context: context(f),
                signed: Box::new(f.signed.clone()),
            },
        )
        .await
    {
        ChatFrame::Issued {
            context: received,
            challenge,
        } => {
            assert_eq!(received, context(f));
            challenge
        }
        other => panic!("genuine challenge not issued: {other:?}"),
    }
}
pub fn post_frame(f: &Fixture, challenge: IssuedChallenge) -> ChatFrame {
    ChatFrame::PostProof {
        context: context(f),
        signed: Box::new(f.signed.clone()),
        ticket: challenge.ticket,
        proof: Box::new(proof(&f.alice, f.policy.scope, challenge)),
    }
}
pub fn unknown(f: &Fixture) -> (PrivateVault, SignedMessage) {
    let directory = f.receiver_path.parent().unwrap();
    let vault =
        PrivateVault::create(&directory.join("unknown-private"), &directory.join("saves")).unwrap();
    let noise = TransportIdentity::create(&vault).unwrap();
    let local = vault.create_identity(noise.peer_id().to_bytes()).unwrap();
    let mut signed = f.signed.clone();
    signed.message.author = author(&local);
    signed.message.message = [82; 16];
    signed.signature = local
        .device_key
        .sign(&codec::message_digest(&signed.message).unwrap());
    assert_eq!(
        nf_contract::signatures::verify_digest(
            &local.public.device_key,
            &codec::message_digest(&signed.message).unwrap(),
            &signed.signature
        ),
        Ok(())
    );
    (vault, signed)
}
#[derive(Clone, Copy)]
pub enum BadReply {
    Request,
    Policy,
    Signature,
    OriginalDigest,
}
/// Deliberately malicious receiver control. Real production builder, actual Bob Noise identity,
/// real store-issued challenge and current proof; never a claimed production delivery.
pub async fn malicious(
    receiver: &mut ChatStore,
    outbox: &mut ClientOutbox,
    f: &Fixture,
    bad: BadReply,
) -> Result<OutgoingEntry, PeerError> {
    let identity = TransportIdentity::load(&f.server_vault).unwrap();
    let mut swarm = identity.build_chat().unwrap();
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = loop {
        if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
            break address.with(libp2p::multiaddr::Protocol::P2p(f.server_peer));
        }
    };
    let pin = ChatPeerPin::from_current(&f.state, author(&f.bob)).unwrap();
    let delivery = deliver_pending(outbox, &f.client_vault, &f.state, &pin, address, [81; 16]);
    tokio::pin!(delivery);
    let mut issued: Option<(IssuedChallenge, ConnectionId)> = None;
    loop {
        tokio::select! {
            result = &mut delivery => return result,
            event = swarm.select_next_some() => if let SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message {
                peer, connection_id, message: Message::Request { request, channel, .. } })) = event {
                assert_eq!(peer, f.client_peer);
                let response = match request {
                    ChatFrame::PostChallenge { context: incoming, signed } => {
                        assert_eq!(incoming, context(f)); assert_eq!(*signed, f.signed);
                        match bad {
                            BadReply::Request | BadReply::Policy => {
                                let mut wrong = incoming;
                                if matches!(bad, BadReply::Request) { wrong.request = RequestId::from_bytes([102; 16]); }
                                else { wrong.policy_digest[0] ^= 1; }
                                ChatFrame::Refused { context: wrong, reason: Refusal::Offline }
                            }
                            _ => {
                                let challenge = receiver.issue_challenge(ChallengeRequest::Post { request: incoming.request, message: &signed }, &peer.to_bytes()).unwrap();
                                issued = Some((challenge, connection_id)); ChatFrame::Issued { context: incoming, challenge }
                            }
                        }
                    }
                    ChatFrame::PostProof { context: incoming, signed, ticket, proof } => {
                        let (challenge, original_connection) = issued.take().unwrap();
                        assert_eq!(incoming, context(f)); assert_eq!(*signed, f.signed);
                        assert_eq!(connection_id, original_connection); assert_eq!(ticket, challenge.ticket);
                        assert_eq!(f.state.authorize(&proof, &peer.to_bytes(), &challenge.challenge, 1, ProtectedOperation::Chat), Ok(1));
                        let mut forged = match f.expected().state { OutgoingState::Delivered(signed) => *signed, _ => unreachable!() };
                        match bad {
                            BadReply::Signature => forged.signature[0] ^= 1,
                            BadReply::OriginalDigest => {
                                forged.receipt.signed_message_digest[0] ^= 1;
                                let canonical = forged.to_canonical_bytes().unwrap();
                                let digest: [u8; 32] = Sha256::digest(&canonical[..259]).into();
                                forged.signature = f.bob.device_key.sign(&digest);
                                assert_eq!(nf_contract::signatures::verify_digest(&f.bob.public.device_key, &digest, &forged.signature), Ok(()));
                            }
                            _ => unreachable!(),
                        }
                        ChatFrame::Delivered { context: incoming, signed: Box::new(forged) }
                    }
                    _ => panic!("unexpected malicious-control request"),
                };
                swarm.behaviour_mut().messages.send_response(channel, response).unwrap();
            }
        }
    }
}
/// Actual foreign persisted Noise key behind an address claiming the trusted Bob peer.
pub async fn wrong_noise(
    outbox: &mut ClientOutbox,
    f: &Fixture,
) -> Result<OutgoingEntry, PeerError> {
    let (vault, _) = unknown(f);
    let identity = TransportIdentity::load(&vault).unwrap();
    assert_ne!(identity.peer_id(), f.server_peer);
    let mut swarm = identity.build_chat().unwrap();
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = loop {
        if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
            break address.with(libp2p::multiaddr::Protocol::P2p(f.server_peer));
        }
    };
    let pin = ChatPeerPin::from_current(&f.state, author(&f.bob)).unwrap();
    let delivery = deliver_pending(outbox, &f.client_vault, &f.state, &pin, address, [81; 16]);
    tokio::pin!(delivery);
    loop {
        tokio::select! {
            result = &mut delivery => return result,
            event = swarm.select_next_some() => if let SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message { .. })) = event {
                panic!("wrong physical Noise key received an application Chat request");
            }
        }
    }
}
