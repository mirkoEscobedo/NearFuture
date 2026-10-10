mod chat_peer_delivery_support;
use chat_peer_delivery_support::{Fixture, author};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_store::chat::{
    ChatStore, KnownChatFrontiers,
    outbox::{ClientOutbox, OutgoingEntry, OutgoingState},
};
use nf_transport::{
    PeerError,
    chat::{ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, deliver_pending},
    identity::TransportIdentity,
};
use std::{fs, time::Duration};
async fn drive(
    server: &mut ChatPeerServer,
    outbox: &mut ClientOutbox,
    fixture: &Fixture,
) -> (
    Result<OutgoingEntry, PeerError>,
    Vec<(PeerId, ConnectionId)>,
) {
    let address = loop {
        if let ChatPeerServerEvent::Ready { address, peer } = server.next().await.unwrap() {
            assert_eq!(peer, fixture.server_peer);
            break address;
        }
    };
    let pin = ChatPeerPin::from_current(&fixture.state, author(&fixture.bob)).unwrap();
    let delivery = deliver_pending(
        outbox,
        &fixture.client_vault,
        &fixture.state,
        &pin,
        address,
        [81; 16],
    );
    tokio::pin!(delivery);
    let mut connections = Vec::new();
    loop {
        tokio::select! {
            result = &mut delivery => return (result, connections),
            event = server.next() => {
                if let ChatPeerServerEvent::Connected { peer, connection } = event.unwrap() {
                    connections.push((peer, connection));
                }
            }
        }
    }
}
#[tokio::test(flavor = "current_thread")]
async fn production_noise_chat_delivers_the_first_original_and_reopens_without_duplicate_history() {
    let fixture = Fixture::new();
    let expected = fixture.expected();
    let known = KnownChatFrontiers {
        scope: fixture.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        let receiver =
            ChatStore::create(&fixture.receiver_path, &fixture.policy, &fixture.state).unwrap();
        assert_eq!(receiver.current_membership(), Ok(fixture.state.clone()));
        let mut outbox = ClientOutbox::create(&fixture.outbox_path, &fixture.profile).unwrap();
        let mut pending = expected.clone();
        pending.state = OutgoingState::Pending;
        assert_eq!(
            outbox.enqueue(fixture.request(), &fixture.signed),
            Ok(pending)
        );
        assert_eq!(outbox.known_revision(), Ok(1));
        let mut server =
            ChatPeerServer::new(receiver, &fixture.server_vault, fixture.policy).unwrap();
        let (result, first_connections) = drive(&mut server, &mut outbox, &fixture).await;
        assert_eq!(first_connections.len(), 1);
        assert_eq!(first_connections[0].0, fixture.client_peer);
        // Intended business RED: actual production Noise challenge/current proof route returns Unsupported before posting.
        assert_eq!(result, Ok(expected.clone()));
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(outbox.known_revision(), Ok(2));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(expected.clone())));
        drop(receiver);
        drop(outbox);
        let receiver_before = fs::read(&fixture.receiver_path).unwrap();
        let outbox_before = fs::read(&fixture.outbox_path).unwrap();
        assert_eq!(
            TransportIdentity::load(&fixture.client_vault)
                .unwrap()
                .peer_id(),
            fixture.client_peer
        );
        assert_eq!(
            TransportIdentity::load(&fixture.server_vault)
                .unwrap()
                .peer_id(),
            fixture.server_peer
        );
        let receiver =
            ChatStore::open_existing(&fixture.receiver_path, &fixture.policy, known).unwrap();
        let mut outbox =
            ClientOutbox::open_existing(&fixture.outbox_path, &fixture.profile, 2).unwrap();
        assert_eq!(receiver.current_membership(), Ok(fixture.state.clone()));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(expected.clone())));
        assert_eq!(outbox.known_revision(), Ok(2));
        let mut server =
            ChatPeerServer::new(receiver, &fixture.server_vault, fixture.policy).unwrap();
        let (result, retry_connections) = drive(&mut server, &mut outbox, &fixture).await;
        assert_eq!(retry_connections.len(), 1);
        assert_eq!(retry_connections[0].0, fixture.client_peer);
        assert_ne!(retry_connections[0].1, first_connections[0].1);
        assert_eq!(result, Ok(expected.clone()));
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(outbox.known_revision(), Ok(2));
        assert_eq!(outbox.entry([81; 16]), Ok(Some(expected)));
        drop(receiver);
        drop(outbox);
        assert_eq!(fs::read(&fixture.receiver_path).unwrap(), receiver_before);
        assert_eq!(fs::read(&fixture.outbox_path).unwrap(), outbox_before);
    })
    .await
    .expect("bounded production foreground Chat deadline");
}
