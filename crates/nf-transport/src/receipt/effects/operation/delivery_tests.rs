//! Backend identifiers are observed from two real Noise peers, never fabricated completions.
use super::delivery::{Delivery, Origin};
use crate::{
    PeerError,
    receipt::*,
    receipt_effects::{ReceiptBehaviour, ReceiptBehaviourEvent, build_receipt_swarm},
    records::{PeerContext, PeerLimits},
};
use futures::{Stream, future::poll_fn};
use libp2p::{
    PeerId, Swarm,
    request_response::{Event, InboundRequestId, Message, ResponseChannel},
    swarm::{ConnectionId, SwarmEvent},
};
use std::{
    task::Poll,
    time::{Duration, Instant},
};
struct Pair {
    server: Swarm<ReceiptBehaviour>,
    client: Swarm<ReceiptBehaviour>,
    client_connection: Option<ConnectionId>,
}
impl Pair {
    fn new() -> Self {
        Self {
            server: build_receipt_swarm(libp2p::identity::Keypair::generate_ed25519()).unwrap(),
            client: build_receipt_swarm(libp2p::identity::Keypair::generate_ed25519()).unwrap(),
            client_connection: None,
        }
    }
    async fn next(&mut self) -> Option<SwarmEvent<ReceiptBehaviourEvent>> {
        poll_fn(|cx| {
            if let Poll::Ready(Some(event)) = std::pin::Pin::new(&mut self.server).poll_next(cx) {
                return Poll::Ready(Some(event));
            }
            if let Poll::Ready(Some(event)) = std::pin::Pin::new(&mut self.client).poll_next(cx) {
                if let SwarmEvent::ConnectionEstablished { connection_id, .. } = event {
                    self.client_connection = Some(connection_id);
                }
                return Poll::Ready(None);
            }
            Poll::Pending
        })
        .await
    }
    async fn connect(&mut self) {
        self.server
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        let address = loop {
            if let Some(SwarmEvent::NewListenAddr { address, .. }) = self.next().await {
                break address;
            }
        };
        self.client.dial(address).unwrap();
        let mut server_connected = false;
        while !server_connected || self.client_connection.is_none() {
            if let Some(SwarmEvent::ConnectionEstablished { .. }) = self.next().await {
                server_connected = true;
            }
        }
    }
    async fn request(
        &mut self,
        record: ReceiptRecord,
    ) -> (
        PeerId,
        ConnectionId,
        InboundRequestId,
        ResponseChannel<ReceiptRecord>,
    ) {
        let peer = *self.server.local_peer_id();
        self.client
            .behaviour_mut()
            .messages
            .send_request(&peer, record.into());
        loop {
            if let Some(SwarmEvent::Behaviour(ReceiptBehaviourEvent::Messages(Event::Message {
                peer,
                connection_id,
                message:
                    Message::Request {
                        request_id,
                        channel,
                        ..
                    },
            }))) = self.next().await
            {
                return (peer, connection_id, request_id, channel);
            }
        }
    }
    async fn sent(&mut self) -> (PeerId, ConnectionId, InboundRequestId) {
        loop {
            if let Some(SwarmEvent::Behaviour(ReceiptBehaviourEvent::Messages(
                Event::ResponseSent {
                    peer,
                    connection_id,
                    request_id,
                },
            ))) = self.next().await
            {
                return (peer, connection_id, request_id);
            }
        }
    }
}
fn hello() -> ReceiptRecord {
    ReceiptRecord {
        context: PeerContext {
            session: [0; 16],
            scope: nf_identity::model::Scope {
                universe: nf_contract::identity::UniverseId::from_bytes([1; 16]),
                history: nf_contract::identity::HistoryId::from_bytes([2; 16]),
            },
            ruleset: [3; 32],
            content: [4; 32],
        },
        body: ReceiptBody::Hello {
            account: nf_contract::identity::AccountId::from_bytes([5; 16]),
            device: nf_contract::identity::DeviceId::from_bytes([6; 16]),
            nonce: [7; 32],
            required: 1,
            optional: 0,
            offered: PeerLimits::default(),
        },
    }
}
#[tokio::test]
async fn actual_rr_ids_cannot_release_later_custody_or_substitute_signed_bytes() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut pair = Pair::new();
        pair.connect().await;
        let record = hello();
        let (peer, connection, first, channel) = pair.request(record.clone()).await;
        pair.server
            .behaviour_mut()
            .messages
            .send_response(channel, record.clone())
            .unwrap();
        assert_eq!(pair.sent().await, (peer, connection, first));
        let (peer2, connection2, second, channel) = pair.request(record.clone()).await;
        assert_eq!((peer2, connection2), (peer, connection));
        assert_ne!(first, second);
        let mut delivery = Delivery::default();
        let minimum = SourceMinima {
            event: nf_contract::identity::EventSeq(0),
            store_revision: 0,
            membership_revision: 0,
        };
        delivery
            .returned(
                &record,
                PeerLimits::default(),
                Origin {
                    peer,
                    connection,
                    revision: 0,
                    created: Instant::now(),
                    minimum,
                },
            )
            .unwrap();
        delivery.response(second).unwrap();
        let occupied = delivery.pending();
        assert!(!delivery.matches_response(first, peer, connection));
        assert!(!delivery.matches_response(second, *pair.server.local_peer_id(), connection));
        assert!(!delivery.matches_response(second, peer, pair.client_connection.unwrap()));
        assert_eq!(delivery.pending(), occupied);
        let mut changed = record.clone();
        let ReceiptBody::Hello { nonce, .. } = &mut changed.body else {
            panic!("hello")
        };
        nonce[0] ^= 1;
        assert!(matches!(
            delivery.check(&changed, peer, connection, PeerLimits::default()),
            Err(PeerError::Session)
        ));
        assert_eq!(delivery.pending(), occupied);
        pair.server
            .behaviour_mut()
            .messages
            .send_response(channel, record)
            .unwrap();
        let (actual_peer, actual_connection, actual_id) = pair.sent().await;
        assert!(delivery.matches_response(actual_id, actual_peer, actual_connection));
        delivery.clear();
        assert_eq!(delivery.pending(), (0, 0, 0));
        // This checks backend custody only; Hello records here establish no app-policy grant.
    })
    .await
    .expect("owned two-peer custody deadline");
}
