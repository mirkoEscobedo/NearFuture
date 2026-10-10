mod chat_command_query_support;
use chat_command_query_support::{Fixture, Owner, hash, message_bytes};
use nf_contract::identity::RequestId;
use nf_ipc::SessionFence;
use nf_store::chat::{
    ChatStore,
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
fn authenticated_cli_enqueues_then_queries_the_same_owner_without_mutating_the_original() {
    let whole = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    let store_before = fs::read(&f.store_path).unwrap();
    let name = "chat-command-query-first";
    let mut owner = Owner::start(&f, name);
    let runtime = owner.session.runtime_session();
    let fence = SessionFence::new(runtime).unwrap();
    let command = g::EnqueueChat {
        request_id: Some(g::RequestId {
            value: vec![111; 16],
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
        text: "Same owner query original".into(),
    };
    let envelope = |body, profile| g::ControlEnvelope {
        protocol_version: 1,
        runtime_session: Some(g::RuntimeSession { value: runtime }),
        required: Some(g::RequiredSemantics {
            capability_ids: vec![profile],
            schema_ids: vec![profile],
        }),
        body: Some(body),
        transport: None,
    };
    let request = envelope(g::control_envelope::Body::EnqueueChat(command.clone()), 4);
    assert_eq!(
        nf_wire::decode_control(&request.encode_to_vec()).unwrap(),
        request
    );
    let response = owner.exchange(&request.encode_to_vec());
    let enqueued = owner
        .session
        .admit_chat_enqueue_response(&response, &command, &fence)
        .unwrap();
    assert_eq!(enqueued.phase, g::ChatOutgoingPhase::Pending as i32);
    assert_eq!(enqueued.source_sequence, 1);
    assert_eq!(enqueued.outbox_revision, 1);
    assert!(enqueued.receiver_receipt.is_empty());
    let signed = decode_signed_message(&enqueued.signed_message).unwrap();
    assert_eq!(signed.message.scope, f.policy.scope);
    assert_eq!(signed.message.author.account, f.alice.public.account);
    assert_eq!(signed.message.author.device, f.alice.public.device);
    assert_eq!(signed.message.sequence, 1);
    assert_eq!(signed.message.text, command.text);
    assert_ne!(signed.message.message, [0; 16]);
    let mut canonical = message_bytes(&signed.message);
    assert_eq!(
        nf_contract::signatures::verify_digest(
            &f.alice.public.device_key,
            &hash(&canonical),
            &signed.signature
        ),
        Ok(())
    );
    canonical.extend_from_slice(&signed.signature);
    assert_eq!(canonical, enqueued.signed_message);
    assert_eq!(
        enqueued.message_id.as_ref().unwrap().value,
        signed.message.message
    );
    let outbox_before_query = fs::read(&f.outbox_path).unwrap();
    let query = g::QueryChatOutgoing {
        request_id: Some(g::RequestId {
            value: vec![112; 16],
        }),
        principal: command.principal.clone(),
        universe_id: command.universe_id.clone(),
        history_id: command.history_id.clone(),
        original_request_id: command.original_request_id.clone(),
        message_id: enqueued.message_id.clone(),
    };
    let request = envelope(
        g::control_envelope::Body::QueryChatOutgoing(query.clone()),
        3,
    );
    assert_eq!(
        nf_wire::decode_control(&request.encode_to_vec()).unwrap(),
        request
    );
    let response = owner.exchange(&request.encode_to_vec());
    let mut expected = enqueued.clone();
    expected.request_id = query.request_id.clone();
    // Genuine missing capability: authenticated same owner currently returns Unsupported here.
    assert_eq!(
        owner.session.admit_chat_response(&response, &query, &fence),
        Ok(expected)
    );
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before_query);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    owner.finish();
    assert!(!f.vault_path.join(format!("blob-{name}")).exists());
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before_query);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.initial_known()).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    assert_eq!(store.current_membership().unwrap(), f.state);
    assert_eq!(store.known_frontiers().unwrap(), f.initial_known());
    assert_eq!(outbox.known_revision(), Ok(1));
    let original = outbox.entry(signed.message.message).unwrap().unwrap();
    assert_eq!(original.original_request, RequestId::from_bytes([101; 16]));
    assert_eq!(original.signed, signed);
    assert_eq!(original.state, OutgoingState::Pending);
    drop(outbox);
    drop(store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before_query);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    assert!(Instant::now() < whole);
}
