use crate::original_fixture::{Fixture, author};
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    request_response::{Event, Message},
    swarm::{ConnectionId, SwarmEvent},
};
use nf_contract::identity::RequestId;
use nf_identity::{
    model::{DeviceProof, DeviceRevocation},
    private_storage::{LocalIdentity, PrivateVault},
    rotation::revocation_digest,
    signing::device_digest,
};
use nf_store::chat::{
    Channel, ChatStore, HistoryPage, HistoryQuery, IssuedChallenge, KnownChatFrontiers,
    ProofAttempt,
    outbox::{ClientOutbox, OutgoingEntry, OutgoingState},
};
use nf_transport::{
    PeerError,
    chat::{
        ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, WireContext, fetch_history,
        history::HistoryFrame,
        network::{ChatBehaviour, ChatBehaviourEvent},
    },
    identity::TransportIdentity,
};
use sha2::{Digest, Sha256};
use std::fs;
/// Public authenticated setup only. No private SQL writes or fabricated membership.
pub struct Prepared {
    pub receiver: Option<ChatStore>,
    outbox: ClientOutbox,
    pending: OutgoingEntry,
    receiver_before: Vec<u8>,
    outbox_before: Vec<u8>,
    known: KnownChatFrontiers,
    membership: nf_identity::model::MembershipState,
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
            next
        } else {
            f.state.clone()
        };
        let known = KnownChatFrontiers {
            scope: f.policy.scope,
            revision: 0,
            membership_revision: membership.revision,
        };
        let mut outbox = ClientOutbox::create(&f.outbox_path, &f.profile).unwrap();
        let mut pending = f.expected();
        pending.state = OutgoingState::Pending;
        assert_eq!(outbox.enqueue(f.request(), &f.signed), Ok(pending.clone()));
        drop(outbox);
        drop(receiver);
        let receiver_before = fs::read(&f.receiver_path).unwrap();
        let outbox_before = fs::read(&f.outbox_path).unwrap();
        let receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
        Self {
            receiver: Some(receiver),
            outbox,
            pending,
            receiver_before,
            outbox_before,
            known,
            membership,
        }
    }
    pub fn server(&mut self, f: &Fixture) -> ChatPeerServer {
        ChatPeerServer::new(self.receiver.take().unwrap(), &f.server_vault, f.policy).unwrap()
    }
    /// Close EXCLUSIVE owners before byte reads; reopen and independently authorize Bob's empty local history.
    pub fn finish(self, mut receiver: ChatStore, f: &Fixture) {
        assert_eq!(receiver.known_frontiers(), Ok(self.known));
        assert_eq!(receiver.current_membership(), Ok(self.membership.clone()));
        assert_eq!(
            local_history(&mut receiver, f),
            HistoryPage {
                entries: vec![],
                next_cursor: 0
            }
        );
        assert_eq!(self.outbox.known_revision(), Ok(1));
        assert_eq!(self.outbox.entry([81; 16]), Ok(Some(self.pending.clone())));
        drop(receiver);
        drop(self.outbox);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), self.receiver_before);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), self.outbox_before);
        let mut receiver =
            ChatStore::open_existing(&f.receiver_path, &f.policy, self.known).unwrap();
        let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
        assert_eq!(receiver.known_frontiers(), Ok(self.known));
        assert_eq!(receiver.current_membership(), Ok(self.membership));
        assert_eq!(
            local_history(&mut receiver, f),
            HistoryPage {
                entries: vec![],
                next_cursor: 0
            }
        );
        assert_eq!(outbox.known_revision(), Ok(1));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(self.pending)));
        drop(receiver);
        drop(outbox);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), self.receiver_before);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), self.outbox_before);
    }
}
fn local_history(receiver: &mut ChatStore, f: &Fixture) -> HistoryPage {
    let query = HistoryQuery {
        request: RequestId::from_bytes([110; 16]),
        reader: author(&f.bob),
        channel: Channel::General,
        after_cursor: 0,
        limit: 2,
    };
    let issued = receiver
        .issue_challenge(
            nf_store::chat::ChallengeRequest::History(&query),
            &f.bob.public.peer,
        )
        .unwrap();
    let proof = proof(&f.bob, f.policy.scope, issued);
    receiver
        .history(
            &query,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &f.bob.public.peer,
            },
        )
        .unwrap()
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
pub struct RawPeer {
    pub swarm: Swarm<ChatBehaviour>,
    peer: PeerId,
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
            tokio::select! {event=self.swarm.select_next_some()=>match event{SwarmEvent::ConnectionEstablished{peer_id,connection_id,..}=>{assert_eq!(peer_id,expected);client=Some(connection_id);},SwarmEvent::OutgoingConnectionError{error,..}=>panic!("unexpected physical history connect failure: {error}"),_=>{}},event=server.next()=>if let ChatPeerServerEvent::Connected{peer,connection}=event.unwrap(){assert_eq!(peer,self.peer);owner=Some(connection);}}
        }
        let ids = (client.unwrap(), owner.unwrap());
        self.connection = Some(ids);
        ids
    }
    pub async fn disconnect(&mut self, server: &mut ChatPeerServer, expected: PeerId) {
        let (client_id, owner_id) = self.connection.take().unwrap();
        assert!(self.swarm.close_connection(client_id));
        let mut client_closed = false;
        let mut owner_closed = false;
        while !client_closed || !owner_closed {
            tokio::select! {event=self.swarm.select_next_some()=>if let SwarmEvent::ConnectionClosed{peer_id,connection_id,..}=event{assert_eq!(peer_id,expected);assert_eq!(connection_id,client_id);client_closed=true;},event=server.next()=>if let ChatPeerServerEvent::Disconnected{peer,connection}=event.unwrap(){assert_eq!(peer,self.peer);assert_eq!(connection,owner_id);owner_closed=true;}}
        }
    }
}
pub fn unknown(f: &Fixture) -> (PrivateVault, nf_store::chat::Author) {
    let dir = f.receiver_path.parent().unwrap();
    let vault =
        PrivateVault::create(&dir.join("unknown-history-private"), &dir.join("saves")).unwrap();
    let noise = TransportIdentity::create(&vault).unwrap();
    let local = vault.create_identity(noise.peer_id().to_bytes()).unwrap();
    (vault, author(&local))
}
pub fn query(f: &Fixture) -> HistoryQuery {
    HistoryQuery {
        request: RequestId::from_bytes([111; 16]),
        reader: author(&f.alice),
        channel: Channel::General,
        after_cursor: 0,
        limit: 1,
    }
}
pub fn context(f: &Fixture, q: HistoryQuery) -> WireContext {
    let mut p = b"NF-CHAT-POLICY-1\0".to_vec();
    p.extend_from_slice(f.policy.scope.universe.as_bytes());
    p.extend_from_slice(f.policy.scope.history.as_bytes());
    p.push(1);
    for n in [2048u16, 4096, 16384, 64] {
        p.extend_from_slice(&n.to_be_bytes());
    }
    WireContext {
        policy_digest: Sha256::digest(p).into(),
        request: q.request,
    }
}
/// Independent literal profile oracle; not used to populate a store or intercept production routing.
pub fn literal(frame: &HistoryFrame) -> Vec<u8> {
    let (c, q) = frame.context_query();
    let tag = match frame {
        HistoryFrame::Challenge { .. } => 1,
        HistoryFrame::Proof { .. } => 2,
        HistoryFrame::Issued { .. } => 3,
        HistoryFrame::Page { .. } => 4,
        HistoryFrame::Refused { .. } => 5,
    };
    let mut b = b"NF-CHAT-HISTORY-WIRE-1\0".to_vec();
    b.push(tag);
    b.extend_from_slice(&c.policy_digest);
    b.extend_from_slice(q.request.as_bytes());
    b.extend_from_slice(q.reader.account.as_bytes());
    b.extend_from_slice(q.reader.device.as_bytes());
    b.push(1);
    b.extend_from_slice(&q.after_cursor.to_be_bytes());
    b.extend_from_slice(&q.limit.to_be_bytes());
    match frame {
        HistoryFrame::Challenge { .. } => {}
        HistoryFrame::Proof { ticket, proof, .. } => {
            b.extend_from_slice(ticket);
            b.extend_from_slice(proof.scope.universe.as_bytes());
            b.extend_from_slice(proof.scope.history.as_bytes());
            b.extend_from_slice(proof.account.as_bytes());
            b.extend_from_slice(proof.device.as_bytes());
            b.extend_from_slice(&proof.frontier.to_be_bytes());
            b.extend_from_slice(&(proof.peer.len() as u16).to_be_bytes());
            b.extend_from_slice(&proof.peer);
            b.extend_from_slice(&proof.challenge);
            b.extend_from_slice(&proof.signature);
        }
        HistoryFrame::Issued { challenge, .. } => {
            b.extend_from_slice(&challenge.ticket);
            b.extend_from_slice(&challenge.challenge);
            b.extend_from_slice(&challenge.membership_revision.to_be_bytes());
        }
        HistoryFrame::Page { page, .. } => {
            b.extend_from_slice(&page.next_cursor.to_be_bytes());
            b.push(page.entries.len() as u8);
            for entry in &page.entries {
                b.extend_from_slice(&entry.receiver_cursor.to_be_bytes());
                let m = &entry.signed.message;
                let mut s = b"NF-CHAT-MESSAGE-1\0".to_vec();
                s.extend_from_slice(m.scope.universe.as_bytes());
                s.extend_from_slice(m.scope.history.as_bytes());
                s.push(1);
                s.extend_from_slice(m.author.account.as_bytes());
                s.extend_from_slice(m.author.device.as_bytes());
                s.extend_from_slice(&m.message);
                s.extend_from_slice(&m.sequence.to_be_bytes());
                s.extend_from_slice(&(m.text.len() as u16).to_be_bytes());
                s.extend_from_slice(m.text.as_bytes());
                s.extend_from_slice(&entry.signed.signature);
                b.extend_from_slice(&(s.len() as u16).to_be_bytes());
                b.extend_from_slice(&s);
            }
        }
        HistoryFrame::Refused { reason, .. } => b.push(match reason {
            nf_transport::chat::Refusal::Unsupported => 1,
            nf_transport::chat::Refusal::Unauthorized => 2,
            nf_transport::chat::Refusal::Limit => 3,
            nf_transport::chat::Refusal::Replay => 4,
            nf_transport::chat::Refusal::Conflict => 5,
            nf_transport::chat::Refusal::Offline => 6,
        }),
    }
    b
}
pub async fn exchange(
    raw: &mut RawPeer,
    server: &mut nf_transport::chat::ChatPeerServer,
    peer: PeerId,
    connection: ConnectionId,
    frame: HistoryFrame,
) -> HistoryFrame {
    let id = raw.swarm.behaviour_mut().history.send_request(&peer, frame);
    loop {
        tokio::select! {event=raw.swarm.select_next_some()=>match event {
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::Message{peer:observed,connection_id,message:Message::Response{request_id,response}}))=>{assert_eq!(observed,peer);assert_eq!(connection_id,connection);assert_eq!(request_id,id);return response;},
            SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::OutboundFailure{error,..}))=>panic!("unexpected history exchange failure: {error}"), _=>{}
        },event=server.next()=>{event.unwrap();}}
    }
}
pub async fn issued(
    raw: &mut RawPeer,
    server: &mut nf_transport::chat::ChatPeerServer,
    f: &Fixture,
    connection: ConnectionId,
    q: HistoryQuery,
) -> IssuedChallenge {
    match exchange(
        raw,
        server,
        f.server_peer,
        connection,
        HistoryFrame::Challenge {
            context: context(f, q),
            query: q,
        },
    )
    .await
    {
        HistoryFrame::Issued {
            context: received,
            query: received_query,
            challenge,
        } => {
            assert_eq!(received, context(f, q));
            assert_eq!(received_query, q);
            assert_eq!(challenge.membership_revision, 1);
            challenge
        }
        other => panic!("actual history challenge not issued: {other:?}"),
    }
}
pub fn proof_frame(f: &Fixture, q: HistoryQuery, c: IssuedChallenge) -> HistoryFrame {
    HistoryFrame::Proof {
        context: context(f, q),
        query: q,
        ticket: c.ticket,
        proof: Box::new(proof(&f.alice, f.policy.scope, c)),
    }
}
#[derive(Clone, Copy)]
pub enum BadPage {
    Request,
    Cursor,
    Signature,
}
/// Explicitly adversarial receiver fixture on real Bob Noise. No production-page-success claim.
/// It produces no SQL post/history writes and never substitutes the real foreground owner's routing.
pub async fn malicious(
    state: &mut Prepared,
    f: &Fixture,
    bad: BadPage,
) -> Result<HistoryPage, PeerError> {
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
    let q = query(f);
    let pin = ChatPeerPin::from_current(&f.state, author(&f.bob)).unwrap();
    let operation = fetch_history(&f.client_vault, &f.state, &pin, address, q);
    tokio::pin!(operation);
    let mut admission = None;
    loop {
        tokio::select! {result=&mut operation=>return result,event=swarm.select_next_some()=>if let SwarmEvent::Behaviour(ChatBehaviourEvent::History(Event::Message{peer,connection_id,message:Message::Request{request,channel,..}}))=event {
            assert_eq!(peer,f.client_peer);
            let reply=match request {
                HistoryFrame::Challenge{context:incoming,query:received}=>{assert_eq!(incoming,context(f,q));assert_eq!(received,q);let challenge=state.receiver.as_mut().unwrap().issue_challenge(nf_store::chat::ChallengeRequest::History(&q),&peer.to_bytes()).unwrap();admission=Some((challenge,connection_id));HistoryFrame::Issued{context:incoming,query:q,challenge}},
                HistoryFrame::Proof{context:incoming,query:received,ticket,proof}=>{let (challenge,original_connection)=admission.take().unwrap();assert_eq!(incoming,context(f,q));assert_eq!(received,q);assert_eq!(connection_id,original_connection);assert_eq!(ticket,challenge.ticket);assert_eq!(f.state.authorize(&proof,&peer.to_bytes(),&challenge.challenge,1,nf_identity::model::ProtectedOperation::Chat),Ok(1));
                    let mut returned=q;let mut c=incoming;
                    if matches!(bad,BadPage::Request) {returned.request=RequestId::from_bytes([112;16]);c.request=returned.request;}
                    if matches!(bad,BadPage::Cursor) {returned.after_cursor=1;}
                    let page=if matches!(bad,BadPage::Signature) {let mut signed=f.signed.clone();signed.signature[0]^=1;HistoryPage{entries:vec![nf_store::chat::HistoryEntry{receiver_cursor:1,signed}],next_cursor:1}}else {HistoryPage{entries:vec![],next_cursor:returned.after_cursor}};
                    HistoryFrame::Page{context:c,query:returned,page:Box::new(page)}
                },_=>panic!("unexpected adversarial history request"),
            };swarm.behaviour_mut().history.send_response(channel,reply).unwrap();
        }}
    }
}
