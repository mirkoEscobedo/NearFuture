#[path = "chat_peer_delivery_support/mod.rs"]
mod chat_peer_delivery_support;
use chat_peer_delivery_support::{Fixture, author};
use libp2p::{PeerId, swarm::ConnectionId};
use nf_contract::identity::RequestId;
use nf_store::chat::{
    Channel, ChatStore, HistoryEntry, HistoryPage, HistoryQuery, KnownChatFrontiers,
    outbox::{ClientOutbox, OutgoingState},
};
use nf_transport::{
    PeerError,
    chat::{
        ChatPeerPin, ChatPeerServer, ChatPeerServerEvent, WireContext, deliver_pending,
        fetch_history,
        history::{self, HistoryFrame},
    },
    identity::TransportIdentity,
};
use std::{fs, future::Future, time::Duration};
async fn address(server: &mut ChatPeerServer, expected: PeerId) -> libp2p::Multiaddr {
    loop {
        if let ChatPeerServerEvent::Ready { address, peer } = server.next().await.unwrap() {
            assert_eq!(peer, expected);
            return address;
        }
    }
}
async fn pump<T>(
    server: &mut ChatPeerServer,
    operation: impl Future<Output = Result<T, PeerError>>,
) -> (Result<T, PeerError>, Vec<(PeerId, ConnectionId)>) {
    tokio::pin!(operation);
    let mut connections = Vec::new();
    loop {
        tokio::select! {
            result = &mut operation => return (result, connections),
            event = server.next() => {
                if let ChatPeerServerEvent::Connected { peer, connection } = event.unwrap() {
                    connections.push((peer, connection));
                }
            }
        }
    }
}
/// Literal independent profile encoder: never injected into a store or a response.
fn expected_page_bytes(f: &Fixture, query: HistoryQuery) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut policy = b"NF-CHAT-POLICY-1\0".to_vec();
    policy.extend_from_slice(f.policy.scope.universe.as_bytes());
    policy.extend_from_slice(f.policy.scope.history.as_bytes());
    policy.push(1);
    for length in [2048u16, 4096, 16384, 64] {
        policy.extend_from_slice(&length.to_be_bytes());
    }
    let mut out = b"NF-CHAT-HISTORY-WIRE-1\0".to_vec();
    out.push(4);
    out.extend_from_slice(&Sha256::digest(policy));
    out.extend_from_slice(query.request.as_bytes());
    out.extend_from_slice(query.reader.account.as_bytes());
    out.extend_from_slice(query.reader.device.as_bytes());
    out.push(1);
    out.extend_from_slice(&0u64.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&1u64.to_be_bytes());
    out.push(1);
    out.extend_from_slice(&1u64.to_be_bytes());
    let m = &f.signed.message;
    let mut signed = b"NF-CHAT-MESSAGE-1\0".to_vec();
    signed.extend_from_slice(m.scope.universe.as_bytes());
    signed.extend_from_slice(m.scope.history.as_bytes());
    signed.push(1);
    signed.extend_from_slice(m.author.account.as_bytes());
    signed.extend_from_slice(m.author.device.as_bytes());
    signed.extend_from_slice(&m.message);
    signed.extend_from_slice(&m.sequence.to_be_bytes());
    signed.extend_from_slice(&(m.text.len() as u16).to_be_bytes());
    signed.extend_from_slice(m.text.as_bytes());
    signed.extend_from_slice(&f.signed.signature);
    out.extend_from_slice(&(signed.len() as u16).to_be_bytes());
    out.extend_from_slice(&signed);
    out
}
#[tokio::test(flavor = "current_thread")]
async fn production_noise_permitted_history_returns_only_the_delivered_cursor_and_reopens_unchanged()
 {
    let f = Fixture::new();
    let delivered = f.expected();
    let known = KnownChatFrontiers {
        scope: f.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    let pin = ChatPeerPin::from_current(&f.state, author(&f.bob)).unwrap();
    let query = HistoryQuery {
        request: RequestId::from_bytes([111; 16]),
        reader: author(&f.alice),
        channel: Channel::General,
        after_cursor: 0,
        limit: 1,
    };
    let expected = HistoryPage {
        entries: vec![HistoryEntry {
            receiver_cursor: 1,
            signed: f.signed.clone(),
        }],
        next_cursor: 1,
    };
    let context = WireContext {
        policy_digest: nf_store::chat::codec::policy_digest(&f.policy),
        request: query.request,
    };
    let frame = HistoryFrame::Page {
        context,
        query,
        page: Box::new(expected.clone()),
    };
    let literal = expected_page_bytes(&f, query);
    assert_eq!(literal.len(), 325);
    assert_eq!(history::encode(&frame), Ok(literal.clone()));
    assert_eq!(history::decode(&literal), Ok(frame));
    tokio::time::timeout(Duration::from_secs(20), async {
        let receiver = ChatStore::create(&f.receiver_path, &f.policy, &f.state).unwrap();
        let mut outbox = ClientOutbox::create(&f.outbox_path, &f.profile).unwrap();
        let mut pending = delivered.clone();
        pending.state = OutgoingState::Pending;
        assert_eq!(outbox.enqueue(f.request(), &f.signed), Ok(pending));
        let mut server = ChatPeerServer::new(receiver, &f.server_vault, f.policy).unwrap();
        let dial = address(&mut server, f.server_peer).await;
        let (posted, post_connections) = pump(
            &mut server,
            deliver_pending(&mut outbox, &f.client_vault, &f.state, &pin, dial, [81; 16]),
        )
        .await;
        assert_eq!(posted, Ok(delivered.clone()));
        assert_eq!(post_connections.len(), 1);
        assert_eq!(post_connections[0].0, f.client_peer);
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(outbox.known_revision(), Ok(2));
        drop(receiver);
        drop(outbox);
        let receiver_bytes = fs::read(&f.receiver_path).unwrap();
        let outbox_bytes = fs::read(&f.outbox_path).unwrap();
        let receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let mut server = ChatPeerServer::new(receiver, &f.server_vault, f.policy).unwrap();
        let dial = address(&mut server, f.server_peer).await;
        let (page, history_connections) = pump(
            &mut server,
            fetch_history(&f.client_vault, &f.state, &pin, dial, query),
        )
        .await;
        assert_eq!(history_connections.len(), 1);
        assert_eq!(history_connections[0].0, f.client_peer);
        assert_ne!(history_connections[0].1, post_connections[0].1);
        // Genuine intended first RED: authenticated current physical reader gets Unsupported instead of the exact source page.
        assert_eq!(page, Ok(expected.clone()));
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        assert_eq!(receiver.current_membership(), Ok(f.state.clone()));
        drop(receiver);
        let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
        assert_eq!(outbox.entry([81; 16]), Ok(Some(delivered.clone())));
        assert_eq!(outbox.known_revision(), Ok(2));
        drop(outbox);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), receiver_bytes);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_bytes);
        assert_eq!(
            TransportIdentity::load(&f.client_vault).unwrap().peer_id(),
            f.client_peer
        );
        assert_eq!(
            TransportIdentity::load(&f.server_vault).unwrap().peer_id(),
            f.server_peer
        );
        let receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let mut server = ChatPeerServer::new(receiver, &f.server_vault, f.policy).unwrap();
        let dial = address(&mut server, f.server_peer).await;
        let (retry, retry_connections) = pump(
            &mut server,
            fetch_history(&f.client_vault, &f.state, &pin, dial, query),
        )
        .await;
        assert_eq!(retry_connections.len(), 1);
        assert_eq!(retry_connections[0].0, f.client_peer);
        assert_ne!(retry_connections[0].1, history_connections[0].1);
        assert_eq!(retry, Ok(expected));
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        drop(receiver);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), receiver_bytes);
        let receiver = ChatStore::open_existing(&f.receiver_path, &f.policy, known).unwrap();
        let mut server = ChatPeerServer::new(receiver, &f.server_vault, f.policy).unwrap();
        let dial = address(&mut server, f.server_peer).await;
        let after = HistoryQuery {
            request: RequestId::from_bytes([112; 16]),
            after_cursor: 1,
            ..query
        };
        let (end, end_connections) = pump(
            &mut server,
            fetch_history(&f.client_vault, &f.state, &pin, dial, after),
        )
        .await;
        assert_eq!(end_connections.len(), 1);
        assert_eq!(end_connections[0].0, f.client_peer);
        assert_ne!(end_connections[0].1, retry_connections[0].1);
        assert_eq!(
            end,
            Ok(HistoryPage {
                entries: vec![],
                next_cursor: 1
            })
        );
        let receiver = server.into_store();
        assert_eq!(receiver.known_frontiers(), Ok(known));
        drop(receiver);
        assert_eq!(fs::read(&f.receiver_path).unwrap(), receiver_bytes);
        assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_bytes);
    })
    .await
    .expect("bounded actual production history workflow");
}
