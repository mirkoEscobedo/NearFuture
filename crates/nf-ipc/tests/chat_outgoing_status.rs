mod chat_outgoing_status_support;
use chat_outgoing_status_support::{Fixture, connect, receive};
use nf_ipc::{ChatQueryPort, EndpointRole, NodeServer, PublishedRendezvous, SessionFence};
use nf_store::chat::{
    ChatStore,
    outbox::{ClientOutbox, OutgoingState},
};
use nf_wire::generated as g;
use prost::Message;
use std::{
    fs,
    time::{Duration, Instant},
};
#[test]
fn authenticated_physical_ipc_reports_pending_then_the_same_genuine_delivered_original_and_reopens()
{
    let until = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    f.create_pending();
    let initial_store = fs::read(&f.store_path).unwrap();
    let initial_outbox = fs::read(&f.outbox_path).unwrap();
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    let port =
        ChatQueryPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1).unwrap();
    let principal = port.principal();
    assert_eq!(principal.account, f.alice.public.account);
    assert_eq!(principal.device, f.alice.public.device);
    assert_eq!(port.known_frontiers().unwrap(), f.initial_known());
    let config = f.config(9);
    let mut node = NodeServer::bind(config.clone(), principal, port).unwrap();
    let publication = PublishedRendezvous::publish(
        &f.vault,
        "chat-status-first",
        config,
        principal,
        node.address(),
    )
    .unwrap();
    node.install_authenticator(
        publication
            .record()
            .authenticator(EndpointRole::Control)
            .unwrap(),
    )
    .unwrap();
    let (mut client, session) = connect(&mut node, &publication);
    assert_eq!(session.runtime_session(), 9);
    let query = f.query(111);
    let envelope = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 9 }),
        required: Some(g::RequiredSemantics {
            capability_ids: vec![3],
            schema_ids: vec![3],
        }),
        body: Some(g::control_envelope::Body::QueryChatOutgoing(query.clone())),
        transport: None,
    };
    let encoded = envelope.encode_to_vec();
    assert_eq!(nf_wire::decode_control(&encoded).unwrap(), envelope);
    client.send(&encoded).unwrap();
    let raw = receive(&mut node, &mut client);
    // First genuine business assertion: initial ChatQueryPort returns Unsupported after validating this original.
    assert_eq!(
        session.admit_chat_response(&raw, &query, &SessionFence::new(9).unwrap()),
        Ok(f.expected(&query, g::ChatOutgoingPhase::Pending, 1, vec![]))
    );
    drop(client);
    node.invalidate();
    assert_eq!(node.connection_count(), 0);
    let (store, outbox) = node.into_port().into_parts();
    publication.close().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), f.initial_known());
    assert_eq!(outbox.known_revision().unwrap(), 1);
    assert_eq!(outbox.entry([81; 16]).unwrap(), Some(f.pending()));
    drop(outbox);
    drop(store);
    assert_eq!(fs::read(&f.store_path).unwrap(), initial_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), initial_outbox);
    assert!(Instant::now() < until);
    let mut store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    let delivered = f.deliver_locally(&mut store, &mut outbox);
    assert!(matches!(&delivered.state, OutgoingState::Delivered(_)));
    assert_eq!(delivered.original_request, f.request());
    assert_eq!(delivered.signed, f.signed);
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 1);
    assert_eq!(known.membership_revision, 1);
    assert_eq!(outbox.known_revision().unwrap(), 2);
    drop(outbox);
    drop(store);
    let delivered_store = fs::read(&f.store_path).unwrap();
    let delivered_outbox = fs::read(&f.outbox_path).unwrap();
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
    let port =
        ChatQueryPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1).unwrap();
    let config = f.config(10);
    let mut node = NodeServer::bind(config.clone(), principal, port).unwrap();
    let publication = PublishedRendezvous::publish(
        &f.vault,
        "chat-status-reopen",
        config,
        principal,
        node.address(),
    )
    .unwrap();
    node.install_authenticator(
        publication
            .record()
            .authenticator(EndpointRole::Control)
            .unwrap(),
    )
    .unwrap();
    let (mut client, session) = connect(&mut node, &publication);
    assert_eq!(session.runtime_session(), 10);
    let query = f.query(112);
    let envelope = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: 10 }),
        required: Some(g::RequiredSemantics {
            capability_ids: vec![3],
            schema_ids: vec![3],
        }),
        body: Some(g::control_envelope::Body::QueryChatOutgoing(query.clone())),
        transport: None,
    };
    client.send(&envelope.encode_to_vec()).unwrap();
    let raw = receive(&mut node, &mut client);
    assert_eq!(
        session.admit_chat_response(&raw, &query, &SessionFence::new(10).unwrap()),
        Ok(f.expected(
            &query,
            g::ChatOutgoingPhase::Delivered,
            2,
            f.expected_receipt()
        ))
    );
    drop(client);
    node.invalidate();
    assert_eq!(node.connection_count(), 0);
    let (store, outbox) = node.into_port().into_parts();
    publication.close().unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(store.current_membership().unwrap(), f.state);
    assert_eq!(outbox.entry([81; 16]).unwrap(), Some(delivered.clone()));
    assert_eq!(outbox.known_revision().unwrap(), 2);
    drop(outbox);
    drop(store);
    assert_eq!(fs::read(&f.store_path).unwrap(), delivered_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), delivered_outbox);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert_eq!(outbox.entry([81; 16]).unwrap(), Some(delivered));
    assert_eq!(outbox.known_revision().unwrap(), 2);
    drop(outbox);
    drop(store);
    assert_eq!(fs::read(&f.store_path).unwrap(), delivered_store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), delivered_outbox);
    assert!(Instant::now() < until);
}
