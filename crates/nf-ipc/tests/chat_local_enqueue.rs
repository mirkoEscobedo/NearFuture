mod chat_local_enqueue_support;
use chat_local_enqueue_support::{Fixture, connect, hash, message_bytes, receive};
use nf_contract::identity::RequestId;
use nf_ipc::{ChatCommandPort, EndpointRole, NodeServer, PublishedRendezvous, SessionFence};
use nf_store::chat::{
    Author, ChatStore,
    codec::decode_signed_message,
    outbox::{ClientOutbox, OutgoingState},
};
use nf_wire::generated as g;
use prost::Message;
use std::{
    fs,
    time::{Duration, Instant},
};
#[test]
fn authenticated_physical_enqueue_allocates_a_durable_original_and_exact_retry_after_reopen() {
    let until = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    let store_before = fs::read(&f.store_path).unwrap();
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    let port = ChatCommandPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1)
        .unwrap();
    let principal = port.principal();
    assert_eq!(principal.account, f.alice.public.account);
    assert_eq!(principal.device, f.alice.public.device);
    let mut first_port = Some(port);
    let mut first_signed = None;
    let mut retained_id = None;
    let mut outbox_before_retry = None;
    for (runtime, query_id) in [(21u64, 111u8), (22, 112)] {
        // Reopen is a fresh owner with the known revision fence.
        let port = if runtime == 21 {
            first_port.take().unwrap()
        } else {
            let store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
            let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
            ChatCommandPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1)
                .unwrap()
        };
        let config = f.config(runtime);
        let mut node = NodeServer::bind(config.clone(), principal, port).unwrap();
        let publication = PublishedRendezvous::publish(
            &f.vault,
            if runtime == 21 {
                "enqueue-first"
            } else {
                "enqueue-reopen"
            },
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
        assert_eq!(session.runtime_session(), runtime);
        let command = g::EnqueueChat {
            request_id: Some(g::RequestId {
                value: vec![query_id; 16],
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
                value: vec![101; 16],
            }),
            channel: 1,
            text: "Durable local enqueue — UTF-8".into(),
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
        let bytes = envelope.encode_to_vec();
        assert_eq!(nf_wire::decode_control(&bytes).unwrap(), envelope);
        client.send(&bytes).unwrap();
        let raw = receive(&mut node, &mut client);
        let admitted = session.admit_chat_enqueue_response(
            &raw,
            &command,
            &SessionFence::new(runtime).unwrap(),
        );
        // Genuine first RED must be Unsupported after authentication/validation, not absent-route or setup failure.
        assert_eq!(
            admitted.as_ref().map(|v| (
                v.phase,
                v.source_sequence,
                v.outbox_revision,
                v.original_request_id.clone()
            )),
            Ok((
                g::ChatOutgoingPhase::Pending as i32,
                1,
                1,
                command.original_request_id.clone()
            ))
        );
        let status = admitted.unwrap();
        assert_eq!(status.request_id, command.request_id);
        assert_eq!(status.principal, command.principal);
        assert_eq!(status.universe_id, command.universe_id);
        assert_eq!(status.history_id, command.history_id);
        assert!(status.receiver_receipt.is_empty());
        let signed = decode_signed_message(&status.signed_message).unwrap();
        assert_eq!(signed.message.scope, f.policy.scope);
        assert_eq!(
            signed.message.author,
            Author {
                account: f.alice.public.account,
                device: f.alice.public.device
            }
        );
        assert_eq!(signed.message.sequence, 1);
        assert_eq!(signed.message.text, command.text);
        assert_ne!(signed.message.message, [0; 16]);
        assert_eq!(
            status.message_id.as_ref().unwrap().value.as_slice(),
            signed.message.message.as_slice()
        );
        let body = message_bytes(&signed.message);
        assert_eq!(signed.signature, f.alice.device_key.sign(&hash(&body)));
        assert_eq!(
            nf_contract::signatures::verify_digest(
                &f.alice.public.device_key,
                &hash(&body),
                &signed.signature
            ),
            Ok(())
        );
        let mut canonical = body;
        canonical.extend_from_slice(&signed.signature);
        assert_eq!(status.signed_message, canonical);
        if let Some(first) = &first_signed {
            assert_eq!(&signed, first);
        }
        if let Some(id) = retained_id {
            assert_eq!(signed.message.message, id);
        }
        retained_id = Some(signed.message.message);
        first_signed = Some(signed.clone());
        drop(client);
        node.invalidate();
        assert_eq!(node.connection_count(), 0);
        let (store, outbox) = node.into_port().into_parts();
        publication.close().unwrap();
        assert_eq!(store.known_frontiers().unwrap(), f.known());
        assert_eq!(outbox.known_revision().unwrap(), 1);
        let entry = outbox.entry(signed.message.message).unwrap().unwrap();
        assert_eq!(entry.original_request, RequestId::from_bytes([101; 16]));
        assert_eq!(entry.signed, signed);
        assert_eq!(entry.state, OutgoingState::Pending);
        drop(outbox);
        drop(store);
        assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
        let durable = fs::read(&f.outbox_path).unwrap();
        if let Some(before) = &outbox_before_retry {
            assert_eq!(&durable, before);
        }
        outbox_before_retry = Some(durable);
        assert!(Instant::now() < until);
    }
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    assert_eq!(outbox.known_revision().unwrap(), 1);
    assert_eq!(
        outbox.entry(retained_id.unwrap()).unwrap().unwrap().signed,
        first_signed.unwrap()
    );
    drop(outbox);
    assert_eq!(
        fs::read(&f.outbox_path).unwrap(),
        outbox_before_retry.unwrap()
    );
}
