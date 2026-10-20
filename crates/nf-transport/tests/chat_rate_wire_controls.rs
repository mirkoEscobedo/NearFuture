mod chat_peer_delivery_support;
use chat_peer_delivery_support::{Fixture, author};
use libp2p::{Multiaddr, swarm::ConnectionId};
use nf_contract::identity::RequestId;
use nf_store::chat::{
    ChatRatePolicy, ChatStore, ReceiverClock,
    outbox::{ClientOutbox, OutgoingEntry, OutgoingState},
};
use nf_transport::{
    PeerError,
    chat::{ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, deliver_pending},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Default)]
struct Clock {
    nanos: AtomicU64,
    reads: AtomicUsize,
}
impl ReceiverClock for Clock {
    fn now(&self) -> Duration {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Duration::from_nanos(self.nanos.load(Ordering::SeqCst))
    }
}
fn retain(root: &Path, name: &str, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
async fn deliver(
    server: &mut ChatPeerServer,
    outbox: &mut ClientOutbox,
    fixture: &Fixture,
    address: &Multiaddr,
    message: [u8; 16],
) -> (Result<OutgoingEntry, PeerError>, ConnectionId, bool) {
    let pin = ChatPeerPin::from_current(&fixture.state, author(&fixture.bob)).unwrap();
    let delivery = deliver_pending(
        outbox,
        &fixture.client_vault,
        &fixture.state,
        &pin,
        address.clone(),
        message,
    );
    tokio::pin!(delivery);
    let mut owned = None;
    let mut closed = false;
    let result = loop {
        tokio::select! {
            result=&mut delivery=>break result,
            event=server.next()=>match event.unwrap(){
                ChatPeerServerEvent::Connected{peer,connection}=>{assert_eq!(peer,fixture.client_peer);assert!(owned.replace(connection).is_none());}
                ChatPeerServerEvent::Disconnected{peer,connection}=>{assert_eq!(peer,fixture.client_peer);assert_eq!(Some(connection),owned);closed=true;}
                _=>{}
            }
        }
    };
    (
        result,
        owned.expect("genuine authenticated client connection"),
        closed,
    )
}
async fn disconnected(
    server: &mut ChatPeerServer,
    owned: ConnectionId,
    fixture: &Fixture,
    closed: bool,
) {
    if closed {
        return;
    }
    loop {
        if let ChatPeerServerEvent::Disconnected { peer, connection } = server.next().await.unwrap()
        {
            assert_eq!(peer, fixture.client_peer);
            assert_eq!(connection, owned);
            return;
        }
    }
}
#[tokio::test(flavor = "current_thread")]
async fn validated_operator_bucket_survives_real_client_reconnect_and_transient_refusal_is_pending()
{
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = external(&fixture, "production-client");
    tokio::time::timeout_at(tokio::time::Instant::from_std(until), async {
        let clock = Arc::new(Clock::default());
        // This explicitly exercises validated LOCAL capacity2; default10 E2E is the independent store FIRST.
        let policy = ChatRatePolicy::new(2, 2_000).unwrap();
        let receiver = ChatStore::create_with_rate_policy_and_clock(
            &fixture.receiver_path,
            &fixture.policy,
            &fixture.state,
            policy,
            clock.clone(),
        )
        .unwrap();
        assert_eq!(receiver.receiver_rate_policy(), Ok((policy, 0)));
        let mut server =
            ChatPeerServer::new(receiver, &fixture.server_vault, fixture.policy).unwrap();
        let address = loop {
            if let ChatPeerServerEvent::Ready { address, peer } = server.next().await.unwrap() {
                assert_eq!(peer, fixture.server_peer);
                break address;
            }
        };
        let mut owners = Vec::new();
        let mut originals = Vec::new();
        let mut outboxes = Vec::new();
        for (index, id) in [81u8, 82, 83].into_iter().enumerate() {
            let mut signed = fixture.signed.clone();
            signed.message.message = [id; 16];
            signed.message.sequence = (index + 1) as u64;
            signed.signature = fixture
                .alice
                .device_key
                .sign(&nf_store::chat::codec::message_digest(&signed.message).unwrap());
            let path = fixture
                .outbox_path
                .with_file_name(format!("rate-outbox-{id}.sqlite"));
            let mut outbox = ClientOutbox::create(&path, &fixture.profile).unwrap();
            let request = RequestId::from_bytes([101 + (index as u8); 16]);
            let pending = outbox.enqueue(request, &signed).unwrap();
            assert_eq!(pending.state, OutgoingState::Pending);
            retain(
                &evidence,
                &format!("signed-{id}.original.bin"),
                &nf_store::chat::codec::encode_signed_message(&signed).unwrap(),
            );
            originals.push(pending);
            outboxes.push((outbox, path));
        }
        for index in 0..2 {
            let (result, owned, closed) = deliver(
                &mut server,
                &mut outboxes[index].0,
                &fixture,
                &address,
                [81 + (index as u8); 16],
            )
            .await;
            let delivered = result.unwrap();
            assert!(matches!(delivered.state, OutgoingState::Delivered(_)));
            assert_eq!(
                delivered.original_request,
                originals[index].original_request
            );
            assert_eq!(delivered.signed, originals[index].signed);
            assert!(owners.iter().all(|old| *old != owned));
            owners.push(owned);
            disconnected(&mut server, owned, &fixture, closed).await;
        }
        let receiver_before = fs::read(&fixture.receiver_path).unwrap();
        retain(&evidence, "receiver-empty.before.sqlite", &receiver_before);
        let outbox_before = fs::read(&outboxes[2].1).unwrap();
        retain(&evidence, "pending-third.before.sqlite", &outbox_before);
        let (result, owned, closed) = deliver(
            &mut server,
            &mut outboxes[2].0,
            &fixture,
            &address,
            [83; 16],
        )
        .await;
        assert_eq!(
            result,
            Err(PeerError::RateLimited {
                retry_after_ms: 2_000
            })
        );
        assert!(owners.iter().all(|old| *old != owned));
        owners.push(owned);
        disconnected(&mut server, owned, &fixture, closed).await;
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(), receiver_before);
        assert_eq!(fs::read(&outboxes[2].1).unwrap(), outbox_before);
        assert_eq!(
            outboxes[2].0.entry([83; 16]),
            Ok(Some(originals[2].clone()))
        );
        assert_eq!(outboxes[2].0.known_revision(), Ok(1)); // No durable terminal refusal or delivered ACK.
        let reads = clock.reads.load(Ordering::SeqCst);
        let first_before = fs::read(&outboxes[0].1).unwrap();
        let (retry, owned, closed) = deliver(
            &mut server,
            &mut outboxes[0].0,
            &fixture,
            &address,
            [81; 16],
        )
        .await;
        assert!(matches!(retry.unwrap().state, OutgoingState::Delivered(_)));
        assert!(owners.iter().all(|old| *old != owned));
        owners.push(owned);
        disconnected(&mut server, owned, &fixture, closed).await;
        assert_eq!(clock.reads.load(Ordering::SeqCst), reads);
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(), receiver_before);
        assert_eq!(fs::read(&outboxes[0].1).unwrap(), first_before);
        clock.nanos.store(2_000_000_000, Ordering::SeqCst);
        let (retried, owned, closed) = deliver(
            &mut server,
            &mut outboxes[2].0,
            &fixture,
            &address,
            [83; 16],
        )
        .await;
        let retried = retried.unwrap();
        assert!(matches!(retried.state, OutgoingState::Delivered(_)));
        assert_eq!(retried.original_request, originals[2].original_request);
        assert_eq!(retried.signed, originals[2].signed);
        assert!(owners.iter().all(|old| *old != owned));
        disconnected(&mut server, owned, &fixture, closed).await;
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers().unwrap().revision, 3);
        assert_eq!(outboxes[2].0.known_revision(), Ok(2));
        retain(
            &evidence,
            "receiver.after-retry.sqlite",
            &fs::read(&fixture.receiver_path).unwrap(),
        );
        retain(
            &evidence,
            "third.after-retry.sqlite",
            &fs::read(&outboxes[2].1).unwrap(),
        );
    })
    .await
    .expect("INSUFFICIENT_EVIDENCE_AUTHENTIC_WIRE_CASE20_DEADLINE");
    assert!(Instant::now() < until);
}

use futures::StreamExt;
use libp2p::{
    PeerId, Swarm,
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_identity::{model::DeviceProof, private_storage::PrivateVault, signing::device_digest};
use nf_store::chat::codec as store_codec;
use nf_transport::{
    chat::{
        ChatFrame, WireContext,
        framing::ChatCodec,
        network::{ChatBehaviour, ChatBehaviourEvent},
    },
    identity::TransportIdentity,
};
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
}

fn context(f: &Fixture, id: u8) -> WireContext {
    WireContext {
        policy_digest: store_codec::policy_digest(&f.policy),
        request: RequestId::from_bytes([id; 16]),
    }
}
async fn submit(raw: &mut RawPeer, server: &mut ChatPeerServer, f: &Fixture, id: u8) -> ChatFrame {
    let context = context(f, id);
    let issued = match raw
        .exchange(
            server,
            f.server_peer,
            ChatFrame::PostChallenge {
                context,
                signed: Box::new(f.signed.clone()),
            },
        )
        .await
    {
        ChatFrame::Issued {
            context: received,
            challenge,
        } => {
            assert_eq!(received, context);
            assert_eq!(challenge.membership_revision, f.state.revision);
            challenge
        }
        other => panic!("fresh authentic challenge refused: {other:?}"),
    };
    let mut proof = DeviceProof {
        scope: f.policy.scope,
        account: f.alice.public.account,
        device: f.alice.public.device,
        frontier: issued.membership_revision,
        peer: f.alice.public.peer.clone(),
        challenge: issued.challenge,
        signature: [0; 64],
    };
    proof.signature = f.alice.device_key.sign(&device_digest(&proof).unwrap());
    raw.exchange(
        server,
        f.server_peer,
        ChatFrame::PostProof {
            context,
            signed: Box::new(f.signed.clone()),
            ticket: issued.ticket,
            proof: Box::new(proof),
        },
    )
    .await
}
fn delivered(frame: ChatFrame, f: &Fixture, id: u8) {
    match frame {
        ChatFrame::Delivered {
            context: received,
            signed,
        } => {
            assert_eq!(received, context(f, id));
            let expected = match f.expected().state {
                OutgoingState::Delivered(signed) => signed,
                _ => unreachable!(),
            };
            assert_eq!(signed, expected);
        }
        other => panic!("genuine delivery missing: {other:?}"),
    }
}
fn external(f: &Fixture, case: &str) -> PathBuf {
    let parent = PathBuf::from(
        std::env::var_os("NF_CHAT_RATE_WIRE_CONTROLS_EVIDENCE_DIR")
            .expect("external wire evidence parent"),
    );
    assert!(parent.is_absolute() && !parent.starts_with(f.receiver_path.parent().unwrap()));
    let root = parent.join(case);
    fs::create_dir(&root).unwrap();
    root
}
#[tokio::test(flavor = "current_thread")]
async fn same_connection_fresh_alias_and_reconnect_share_one_account_bucket() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = external(&fixture, "same-connection");
    tokio::time::timeout_at(tokio::time::Instant::from_std(until), async {
        let clock = Arc::new(Clock::default());
        let policy = ChatRatePolicy::new(2, 2_000).unwrap();
        let receiver = ChatStore::create_with_rate_policy_and_clock(
            &fixture.receiver_path,
            &fixture.policy,
            &fixture.state,
            policy,
            clock.clone(),
        )
        .unwrap();
        assert_eq!(receiver.receiver_rate_policy(), Ok((policy, 0)));
        let mut server =
            ChatPeerServer::new(receiver, &fixture.server_vault, fixture.policy).unwrap();
        let address = loop {
            if let ChatPeerServerEvent::Ready { address, peer } = server.next().await.unwrap() {
                assert_eq!(peer, fixture.server_peer);
                break address;
            }
        };
        let mut raw = RawPeer::new(&fixture.client_vault);
        let first = raw
            .connect(&mut server, address.clone(), fixture.server_peer)
            .await;
        delivered(
            submit(&mut raw, &mut server, &fixture, 101).await,
            &fixture,
            101,
        );
        delivered(
            submit(&mut raw, &mut server, &fixture, 102).await,
            &fixture,
            102,
        );
        let before = fs::read(&fixture.receiver_path).unwrap();
        retain(&evidence, "receiver-empty.before.sqlite", &before);
        let refused = submit(&mut raw, &mut server, &fixture, 103).await;
        assert_eq!(
            refused,
            ChatFrame::RateLimited {
                context: context(&fixture, 103),
                retry_after_ms: 2_000
            }
        );
        retain(
            &evidence,
            "received-rate.canonical.bin",
            &nf_transport::chat::codec::encode(&refused).unwrap(),
        );
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(), before);
        let reads = clock.reads.load(Ordering::SeqCst);
        delivered(
            submit(&mut raw, &mut server, &fixture, 101).await,
            &fixture,
            101,
        );
        assert_eq!(clock.reads.load(Ordering::SeqCst), reads);
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(), before);
        assert_eq!(raw.connection, Some(first));
        raw.disconnect(&mut server, fixture.server_peer).await;
        drop(raw);
        let mut raw = RawPeer::new(&fixture.client_vault);
        let second = raw.connect(&mut server, address, fixture.server_peer).await;
        assert_ne!(first.0, second.0);
        assert_ne!(first.1, second.1);
        assert_eq!(
            submit(&mut raw, &mut server, &fixture, 103).await,
            ChatFrame::RateLimited {
                context: context(&fixture, 103),
                retry_after_ms: 2_000
            }
        );
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(), before);
        clock.nanos.store(2_000_000_000, Ordering::SeqCst);
        delivered(
            submit(&mut raw, &mut server, &fixture, 103).await,
            &fixture,
            103,
        );
        raw.disconnect(&mut server, fixture.server_peer).await;
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers().unwrap().revision, 1);
        retain(
            &evidence,
            "receiver-after-refill.sqlite",
            &fs::read(&fixture.receiver_path).unwrap(),
        );
        retain(
            &evidence,
            "signed.original.bin",
            &store_codec::encode_signed_message(&fixture.signed).unwrap(),
        );
    })
    .await
    .expect("INSUFFICIENT_EVIDENCE_AUTHENTIC_SAME_CONNECTION20");
    assert!(Instant::now() < until);
}
fn old_protocol(raw: &mut RawPeer) {
    // Genuine old negotiation profile; payload codec cannot be reached when new peer rejects /chat/1.
    raw.swarm.behaviour_mut().messages = libp2p::request_response::Behaviour::with_codec(
        ChatCodec,
        [(
            libp2p::StreamProtocol::new("/nearfuture/chat/1"),
            libp2p::request_response::ProtocolSupport::Full,
        )],
        libp2p::request_response::Config::default()
            .with_request_timeout(Duration::from_secs(5))
            .with_max_concurrent_streams(1),
    );
}
#[tokio::test(flavor = "current_thread")]
async fn protocol_one_only_noise_peers_are_unsupported_both_directions() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = external(&fixture, "old-peer");
    tokio::time::timeout_at(tokio::time::Instant::from_std(until),async{
        let receiver=ChatStore::create(&fixture.receiver_path,&fixture.policy,&fixture.state).unwrap();
        let before=fs::read(&fixture.receiver_path).unwrap();retain(&evidence,"receiver.before.sqlite",&before);
        let mut server=ChatPeerServer::new(receiver,&fixture.server_vault,fixture.policy).unwrap();
        let address=loop{if let ChatPeerServerEvent::Ready{address,peer}=server.next().await.unwrap(){assert_eq!(peer,fixture.server_peer);break address;}};
        let mut old=RawPeer::new(&fixture.client_vault);old_protocol(&mut old);
        let ids=old.connect(&mut server,address,fixture.server_peer).await;
        let id=old.swarm.behaviour_mut().messages.send_request(&fixture.server_peer,
            ChatFrame::PostChallenge{context:context(&fixture,101),signed:Box::new(fixture.signed.clone())});
        loop{tokio::select!{
            event=old.swarm.select_next_some()=>match event{
                SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::OutboundFailure{peer,connection_id,request_id,error}))=>{
                    assert_eq!(peer,fixture.server_peer);assert_eq!(connection_id,ids.0);assert_eq!(request_id,id);
                    assert!(matches!(error,libp2p::request_response::OutboundFailure::UnsupportedProtocols));break;
                }
                SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message{..}))=>panic!("old protocol reached new application"),
                _=>{}
            },
            event=server.next()=>{event.unwrap();}
        }}
        old.disconnect(&mut server,fixture.server_peer).await;drop(old);
        let receiver=server.into_store();assert_eq!(receiver.known_frontiers().unwrap().revision,0);drop(receiver);
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(),before);
        let mut outbox=ClientOutbox::create(&fixture.outbox_path,&fixture.profile).unwrap();
        let pending=outbox.enqueue(fixture.request(),&fixture.signed).unwrap();assert_eq!(pending.state,OutgoingState::Pending);
        let outbox_before=fs::read(&fixture.outbox_path).unwrap();retain(&evidence,"pending.before.sqlite",&outbox_before);
        let mut old_server=RawPeer::new(&fixture.server_vault);old_protocol(&mut old_server);
        old_server.swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
        let address=loop{if let SwarmEvent::NewListenAddr{address,..}=old_server.swarm.select_next_some().await{
            break address.with(libp2p::multiaddr::Protocol::P2p(fixture.server_peer));
        }};
        let pin=ChatPeerPin::from_current(&fixture.state,author(&fixture.bob)).unwrap();
        let result={
            let delivery=deliver_pending(&mut outbox,&fixture.client_vault,&fixture.state,&pin,address,[81;16]);
            tokio::pin!(delivery);
            loop{tokio::select!{
                result=&mut delivery=>break result,
                event=old_server.swarm.select_next_some()=>if let SwarmEvent::Behaviour(ChatBehaviourEvent::Messages(Event::Message{..}))=event{
                    panic!("new client reached old application; fail-open negotiation");
                }
            }}
        }; // The actual borrowing future and owned client swarm drop at this block boundary.
        assert_eq!(result,Err(PeerError::Unsupported));
        assert_eq!(outbox.entry([81;16]),Ok(Some(pending)));assert_eq!(outbox.known_revision(),Ok(1));
        assert_eq!(fs::read(&fixture.outbox_path).unwrap(),outbox_before);
        retain(&evidence,"receiver.after.sqlite",&fs::read(&fixture.receiver_path).unwrap());
        retain(&evidence,"pending.after.sqlite",&fs::read(&fixture.outbox_path).unwrap());
    }).await.expect("INSUFFICIENT_EVIDENCE_AUTHENTIC_OLD_PEER20");
    assert!(Instant::now() < until);
}
