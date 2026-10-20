//! Test-only adversarial codec writes actual malformed frames under the admitted disposable Noise key.
use super::fixture::Fixture;
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, Stream, future::poll_fn};
use libp2p::{
    PeerId, StreamProtocol, Swarm, SwarmBuilder,
    request_response::{self, Codec, Event, Message, OutboundRequestId},
    swarm::SwarmEvent,
};
use nf_transport::{PeerError, receipt::*, receipt_effects::*, records::PeerLimits};
use std::{io, task::Poll, time::Duration};
#[derive(Clone)]
struct Attack {
    record: ReceiptRecord,
    truncate: bool,
}
#[derive(Clone, Default)]
struct AttackCodec;
impl Codec for AttackCodec {
    type Protocol = StreamProtocol;
    type Request = Attack;
    type Response = ReceiptRecord;
    async fn read_request<T: AsyncRead + Unpin + Send>(
        &mut self,
        p: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<Attack> {
        Ok(Attack {
            record: ReceiptCodec.read_request(p, io).await?,
            truncate: false,
        })
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(
        &mut self,
        p: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<ReceiptRecord> {
        ReceiptCodec.read_response(p, io).await
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(
        &mut self,
        p: &StreamProtocol,
        io: &mut T,
        r: Attack,
    ) -> io::Result<()> {
        if p.as_ref() != RECEIPT_PROTOCOL {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut bytes = encode_body(&r.record, PeerLimits::default())
            .map_err(|_| io::ErrorKind::InvalidData)?;
        if r.truncate {
            assert!(matches!(
                r.record.body,
                ReceiptBody::ClientProof(_) | ReceiptBody::Prove { .. }
            ));
            bytes.pop();
        }
        io.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
        io.write_all(&bytes).await?;
        io.close().await
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(
        &mut self,
        p: &StreamProtocol,
        io: &mut T,
        r: ReceiptRecord,
    ) -> io::Result<()> {
        ReceiptCodec.write_response(p, io, r).await
    }
}
type AttackSwarm = Swarm<request_response::Behaviour<AttackCodec>>;
fn attacker(f: &Fixture) -> AttackSwarm {
    let vault = nf_identity::private_storage::PrivateVault::open(
        &f.scratch.root.join("client-private"),
        &f.scratch.root.join("saves"),
    )
    .unwrap();
    // Only disposable generated credentials; bounded, zeroized and loaded before protocol deadlines.
    let bytes = zeroize::Zeroizing::new(vault.read_private_blob("transport-ed25519-v1").unwrap());
    assert!(bytes.len() <= 4096);
    let key = libp2p::identity::Keypair::from_protobuf_encoding(&bytes).unwrap();
    assert_eq!(key.public().to_peer_id().to_bytes(), f.client_public.peer);
    SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            || nf_transport::mux::LaneMuxConfig::new(nf_transport::records::Lane::Control),
        )
        .unwrap()
        .with_behaviour(|_| {
            request_response::Behaviour::with_codec(
                AttackCodec,
                [(
                    StreamProtocol::new(RECEIPT_PROTOCOL),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default()
                    .with_request_timeout(Duration::from_secs(5))
                    .with_max_concurrent_streams(4),
            )
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(61)))
        .build()
}
enum Step {
    Server(Result<Option<ReceiptLaneEvent>, PeerError>),
    Client(Box<SwarmEvent<Event<Attack, ReceiptRecord>>>),
}
async fn step(server: &mut ReceiptLane, client: &mut AttackSwarm, f: &mut Fixture) -> Step {
    poll_fn(|cx| {
        if let Poll::Ready(result) = server.poll(cx, &mut f.server) {
            return Poll::Ready(Step::Server(result));
        }
        if let Poll::Ready(Some(event)) = std::pin::Pin::new(&mut *client).poll_next(cx) {
            return Poll::Ready(Step::Client(Box::new(event)));
        }
        Poll::Pending
    })
    .await
}
async fn response(
    server: &mut ReceiptLane,
    client: &mut AttackSwarm,
    f: &mut Fixture,
    id: OutboundRequestId,
) -> ReceiptRecord {
    loop {
        match step(server, client, f).await {
            Step::Server(result) => {
                result.unwrap();
            }
            Step::Client(event) => match *event {
                SwarmEvent::Behaviour(Event::Message {
                    message:
                        Message::Response {
                            request_id,
                            response,
                        },
                    ..
                }) if request_id == id => return response,
                SwarmEvent::Behaviour(Event::OutboundFailure { request_id, .. })
                    if request_id == id =>
                {
                    panic!("valid setup request failed")
                }
                _ => {}
            },
        }
    }
}
async fn malformed_first_attempt(operation: bool) {
    let mut f = Fixture::new();
    let mut client = attacker(&f);
    let mut server = ReceiptLane::server(&f.server).unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        server
            .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let address = loop {
            if let Some(ReceiptLaneEvent::Listening(address)) =
                poll_fn(|cx| server.poll(cx, &mut f.server)).await.unwrap()
            {
                break address;
            }
        };
        client.dial(address).unwrap();
        let connection = loop {
            match step(&mut server, &mut client, &mut f).await {
                Step::Client(event) => {
                    if let SwarmEvent::ConnectionEstablished { connection_id, .. } = *event {
                        break connection_id;
                    }
                }
                Step::Server(result) => {
                    result.unwrap();
                }
            }
        };
        let peer = PeerId::from_bytes(&f.server_public.peer).unwrap();
        let mut handshake = ReceiptClientHandshake::new();
        let hello = handshake.hello(&mut f.client).unwrap();
        let id = client.behaviour_mut().send_request(
            &peer,
            Attack {
                record: hello,
                truncate: false,
            },
        );
        let challenge = response(&mut server, &mut client, &mut f, id).await;
        let proof = handshake
            .challenge(challenge, peer, connection, &mut f.client)
            .unwrap();
        let proof = if operation {
            let id = client.behaviour_mut().send_request(
                &peer,
                Attack {
                    record: proof,
                    truncate: false,
                },
            );
            let finished = response(&mut server, &mut client, &mut f, id).await;
            let session = handshake
                .finished(finished, peer, connection, &mut f.client)
                .unwrap()
                .into_session();
            // The actual server Finished ResponseSent must activate its private operation owner.
            while server.pending_encoded().0 != 0 {
                if let Step::Server(result) = step(&mut server, &mut client, &mut f).await {
                    result.unwrap();
                }
            }
            let (original, minimum) = f.client.original_for_slot(0).unwrap();
            let mut query =
                ReceiptOperationClient::new(session, connection, original, minimum).unwrap();
            let begin = query.begin(&mut f.client).unwrap();
            let id = client.behaviour_mut().send_request(
                &peer,
                Attack {
                    record: begin,
                    truncate: false,
                },
            );
            let challenge = response(&mut server, &mut client, &mut f, id).await;
            query
                .challenge(challenge, peer, connection, &mut f.client)
                .unwrap()
        } else {
            proof
        };
        let bad = client.behaviour_mut().send_request(
            &peer,
            Attack {
                record: proof.clone(),
                truncate: true,
            },
        );
        let mut retried = false;
        loop {
            match step(&mut server, &mut client, &mut f).await {
                Step::Server(Err(PeerError::Offline | PeerError::Malformed)) => break,
                Step::Server(Ok(Some(ReceiptLaneEvent::Authenticated { .. }))) => {
                    panic!("malformed first proof survived and a later proof activated authority")
                }
                Step::Server(result) => {
                    result.unwrap();
                }
                Step::Client(event) => match *event {
                    SwarmEvent::Behaviour(Event::OutboundFailure { request_id, .. })
                        if request_id == bad && !retried =>
                    {
                        // Under the defective owner this later valid proof reveals the surviving challenge.
                        client.behaviour_mut().send_request(
                            &peer,
                            Attack {
                                record: proof.clone(),
                                truncate: false,
                            },
                        );
                        retried = true;
                    }
                    SwarmEvent::Behaviour(Event::Message {
                        message: Message::Response { .. },
                        ..
                    }) => {
                        panic!(
                            "malformed first proof survived and produced a later protected reply"
                        )
                    }
                    _ => {}
                },
            }
        }
        assert!(matches!(
            poll_fn(|cx| server.poll(cx, &mut f.server)).await,
            Err(PeerError::Offline)
        ));
        assert_eq!(server.pending_encoded(), (0, 0, 0));
        assert_eq!(f.client.book_anchors()[0].generation, 0);
    })
    .await
    .expect("owned malformed proof deadline");
}
#[tokio::test]
async fn actual_malformed_client_proof_consumes_the_owned_handshake() {
    malformed_first_attempt(false).await;
}
#[tokio::test]
async fn actual_malformed_operation_proof_consumes_the_owned_challenge() {
    malformed_first_attempt(true).await;
}
