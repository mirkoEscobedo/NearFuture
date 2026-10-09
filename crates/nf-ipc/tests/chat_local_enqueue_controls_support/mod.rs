#[path = "../chat_local_enqueue_support/mod.rs"]
mod frozen;
pub use frozen::{Fixture, hash, message_bytes};
use nf_ipc::{
    ChatCommandPort, EndpointRole, IpcError, NodeServer, PublishedRendezvous, SessionFence,
};
use nf_store::chat::{ChatStore, KnownChatFrontiers, outbox::ClientOutbox};
use nf_wire::generated as g;
use prost::Message;

/// Real bounded loopback mutual-auth route; no mock session, authorization or production bypass.
pub fn submit(
    f: &Fixture,
    outbox_revision: u64,
    known: KnownChatFrontiers,
    runtime: u64,
    original: u8,
    text: &str,
) -> Result<g::ChatOutgoingStatus, IpcError> {
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, outbox_revision).unwrap();
    let port = ChatCommandPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1)
        .unwrap();
    let principal = port.principal();
    let config = f.config(runtime);
    let mut node = NodeServer::bind(config.clone(), principal, port).unwrap();
    let publication = PublishedRendezvous::publish(
        &f.vault,
        "enqueue-controls",
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
    let (mut client, session) = frozen::connect(&mut node, &publication);
    assert_eq!(session.runtime_session(), runtime);
    let command = g::EnqueueChat {
        request_id: Some(g::RequestId {
            value: vec![runtime as u8; 16],
        }),
        principal: Some(g::Principal {
            account_id: Some(g::AccountId {
                value: f.alice.public.account.as_bytes().to_vec(),
            }),
            device_id: Some(g::DeviceId {
                value: f.alice.public.device.as_bytes().to_vec(),
            }),
        }),
        universe_id: Some(g::UniverseId { value: vec![1; 16] }),
        history_id: Some(g::HistoryId { value: vec![2; 16] }),
        original_request_id: Some(g::RequestId {
            value: vec![original; 16],
        }),
        channel: 1,
        text: text.into(),
    };
    let envelope = g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: runtime }),
        required: Some(g::RequiredSemantics {
            capability_ids: vec![4],
            schema_ids: vec![4],
        }),
        body: Some(g::control_envelope::Body::EnqueueChat(command.clone())),
        transport: None,
    };
    client.send(&envelope.encode_to_vec()).unwrap();
    let raw = frozen::receive(&mut node, &mut client);
    let result =
        session.admit_chat_enqueue_response(&raw, &command, &SessionFence::new(runtime).unwrap());
    drop(client);
    node.invalidate();
    assert_eq!(node.connection_count(), 0);
    let (store, outbox) = node.into_port().into_parts();
    publication.close().unwrap();
    drop(outbox);
    drop(store);
    result
}
