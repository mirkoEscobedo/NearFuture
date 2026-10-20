mod chat_local_enqueue_controls_support;
use chat_local_enqueue_controls_support::{Fixture, hash, message_bytes, submit};
use nf_contract::identity::RequestId;
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, DeviceRevocation},
    private_storage::PrivateVault,
    rotation::revocation_digest,
    signing::device_digest,
};
use nf_ipc::{ChatCommandPort, IpcError};
use nf_store::chat::{
    Author, ChallengeRequest, Channel, ChatMessage, ChatStore, ChatStoreError, KnownChatFrontiers,
    LocalReceiptIssuer, ProofAttempt, SignedMessage,
    codec::decode_signed_message,
    outbox::{ClientOutbox, OutboxProfile, OutgoingState},
};
use nf_wire::generated as g;
use std::{
    fs,
    time::{Duration, Instant},
};

#[test]
fn divergent_same_original_is_refused_without_sql_effects_or_replacing_the_first() {
    let until = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    let first = submit(&f, 0, f.known(), 31, 101, "first original").unwrap();
    let signed = decode_signed_message(&first.signed_message).unwrap();
    let outbox_before = fs::read(&f.outbox_path).unwrap();
    let store_before = fs::read(&f.store_path).unwrap();
    assert_eq!(
        submit(&f, 1, f.known(), 32, 101, "divergent text"),
        Err(IpcError::Malformed)
    );
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    assert_eq!(outbox.known_revision(), Ok(1));
    let retained = outbox.entry(signed.message.message).unwrap().unwrap();
    assert_eq!(retained.original_request, RequestId::from_bytes([101; 16]));
    assert_eq!(retained.signed, signed);
    assert_eq!(retained.state, OutgoingState::Pending);
    drop(outbox);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
    assert_eq!(store.known_frontiers(), Ok(f.known()));
    drop(store);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before);
    assert!(Instant::now() < until);
}
#[test]
fn distinct_original_allocates_sender_sequence_two_and_preserves_the_first() {
    let until = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    let store_before = fs::read(&f.store_path).unwrap();
    let first = submit(&f, 0, f.known(), 33, 101, "one").unwrap();
    let second = submit(&f, 1, f.known(), 34, 102, "two").unwrap();
    assert_eq!((first.source_sequence, first.outbox_revision), (1, 1));
    assert_eq!((second.source_sequence, second.outbox_revision), (2, 2));
    assert_eq!(second.phase, g::ChatOutgoingPhase::Pending as i32);
    assert_eq!(second.original_request_id.unwrap().value, vec![102; 16]);
    let a = decode_signed_message(&first.signed_message).unwrap();
    let b = decode_signed_message(&second.signed_message).unwrap();
    assert_ne!(a.message.message, b.message.message);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
    assert_eq!(outbox.entry(a.message.message).unwrap().unwrap().signed, a);
    assert_eq!(outbox.entry(b.message.message).unwrap().unwrap().signed, b);
    assert_eq!(outbox.known_revision(), Ok(2));
    for signed in [&a, &b] {
        assert_eq!(
            nf_contract::signatures::verify_digest(
                &f.alice.public.device_key,
                &hash(&message_bytes(&signed.message)),
                &signed.signature
            ),
            Ok(())
        );
    }
    drop(outbox);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    assert!(Instant::now() < until);
}
#[test]
fn delivered_original_retry_returns_the_genuine_receipt_without_new_history_or_revision() {
    let until = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    let first = submit(&f, 0, f.known(), 35, 101, "delivered original").unwrap();
    let signed = decode_signed_message(&first.signed_message).unwrap();
    let request = RequestId::from_bytes([101; 16]);
    let mut store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
    let issued = store
        .issue_challenge(
            ChallengeRequest::Post {
                request,
                message: &signed,
            },
            &f.alice.public.peer,
        )
        .unwrap();
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
    let committed = store
        .post(
            request,
            &signed,
            ProofAttempt {
                ticket: issued.ticket,
                proof: &proof,
                peer: &f.alice.public.peer,
            },
        )
        .unwrap();
    assert_eq!(
        (
            committed.source_sequence,
            committed.receiver_cursor,
            committed.original_request
        ),
        (1, 1, request)
    );
    let directory = f.store_path.parent().unwrap();
    let bob_vault =
        PrivateVault::open(&directory.join("bob-private"), &directory.join("saves")).unwrap();
    let bob = bob_vault.load_identity(vec![42; 32]).unwrap();
    let receipt = store
        .issue_delivery_receipt(
            request,
            LocalReceiptIssuer {
                policy: f.policy,
                receiver: Author {
                    account: bob.public.account,
                    device: bob.public.device,
                },
                peer: &bob.public.peer,
                device_key: &bob.device_key,
            },
        )
        .unwrap();
    let receipt_bytes = receipt.to_canonical_bytes().unwrap();
    assert_eq!(
        nf_contract::signatures::verify_digest(
            &bob.public.device_key,
            &hash(&receipt_bytes[..receipt_bytes.len() - 64]),
            &receipt.signature
        ),
        Ok(())
    );
    let known = KnownChatFrontiers {
        scope: f.policy.scope,
        revision: 1,
        membership_revision: 1,
    };
    assert_eq!(store.known_frontiers(), Ok(known));
    drop(store);
    let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    let delivered = outbox.acknowledge(&receipt).unwrap();
    assert_eq!(
        delivered.state,
        OutgoingState::Delivered(Box::new(receipt.clone()))
    );
    assert_eq!(outbox.known_revision(), Ok(2));
    drop(outbox);
    let store_before = fs::read(&f.store_path).unwrap();
    let outbox_before = fs::read(&f.outbox_path).unwrap();
    let retry = submit(&f, 2, known, 36, 101, "delivered original").unwrap();
    assert_eq!(
        (retry.phase, retry.source_sequence, retry.outbox_revision),
        (g::ChatOutgoingPhase::Delivered as i32, 1, 2)
    );
    assert_eq!(retry.signed_message, first.signed_message);
    assert_eq!(retry.receiver_receipt, receipt_bytes);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 2).unwrap();
    assert_eq!(
        outbox.entry(signed.message.message).unwrap().unwrap(),
        delivered
    );
    drop(outbox);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    assert_eq!(store.known_frontiers(), Ok(known));
    drop(store);
    assert!(Instant::now() < until);
}
#[test]
fn wrong_seed_is_refused_by_public_local_enqueue_without_sql_effects() {
    let f = Fixture::new();
    let before = fs::read(&f.outbox_path).unwrap();
    let wrong = SecretSeed::generate().unwrap();
    assert_ne!(wrong.public_key(), f.alice.public.device_key);
    let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    assert_eq!(
        outbox.enqueue_local(RequestId::from_bytes([101; 16]), "wrong seed", &wrong),
        Err(ChatStoreError::Signature)
    );
    assert_eq!(outbox.known_revision(), Ok(0));
    drop(outbox);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), before);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    assert_eq!(outbox.known_revision(), Ok(0));
}
#[test]
fn current_account_signed_revocation_refuses_fresh_command_owner_without_new_writes() {
    let f = Fixture::new();
    let outbox_before = fs::read(&f.outbox_path).unwrap();
    let mut store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
    let change = DeviceRevocation {
        scope: f.policy.scope,
        issuer: f.alice.public.account,
        device: f.alice.public.device,
        frontier: 1,
    };
    let signature = f.alice.account_key.sign(&revocation_digest(&change));
    let current = store.revoke_device(&change, &signature).unwrap();
    assert_eq!(current.revision, 2);
    assert!(current.devices.get(&f.alice.public.device).unwrap().revoked);
    drop(store);
    // This intentional signed membership write is setup; the following refused owner must have no effects.
    let store_before = fs::read(&f.store_path).unwrap();
    let known = KnownChatFrontiers {
        scope: f.policy.scope,
        revision: 0,
        membership_revision: 2,
    };
    let store = ChatStore::open_existing(&f.store_path, &f.policy, known).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    assert_eq!(
        ChatCommandPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1).err(),
        Some(IpcError::Unauthorized)
    );
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), outbox_before);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    assert_eq!(outbox.known_revision(), Ok(0));
}
#[test]
fn wrong_full_peer_and_valid_foreign_sender_lifetime_pins_refuse_without_sql_effects() {
    let f = Fixture::new();
    let store_before = fs::read(&f.store_path).unwrap();
    let before = fs::read(&f.outbox_path).unwrap();
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    assert_eq!(
        ChatCommandPort::from_vault(store, outbox, &f.vault, vec![43; 32], 1).err(),
        Some(IpcError::Unauthorized)
    );
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), before);
    let store = ChatStore::open_existing(&f.store_path, &f.policy, f.known()).unwrap();
    let state = store.current_membership().unwrap();
    let (bob_id, bob_device) = state
        .devices
        .iter()
        .find(|(_, device)| device.peer == vec![42; 32])
        .unwrap();
    let bob_actor = Author {
        account: bob_device.account,
        device: *bob_id,
    };
    let alice_actor = Author {
        account: f.alice.public.account,
        device: f.alice.public.device,
    };
    let foreign = OutboxProfile::from_current(&f.policy, &state, bob_actor, alice_actor).unwrap();
    let foreign_path = f
        .outbox_path
        .parent()
        .unwrap()
        .join("foreign-outbox.sqlite");
    let outbox = ClientOutbox::create(&foreign_path, &foreign).unwrap();
    drop(outbox);
    let foreign_before = fs::read(&foreign_path).unwrap();
    let outbox = ClientOutbox::open_existing(&foreign_path, &foreign, 0).unwrap();
    assert_eq!(
        ChatCommandPort::from_vault(store, outbox, &f.vault, f.alice.public.peer.clone(), 1).err(),
        Some(IpcError::Unauthorized)
    );
    assert_eq!(fs::read(&foreign_path).unwrap(), foreign_before);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    assert_eq!(fs::read(&f.outbox_path).unwrap(), before);
}
#[test]
fn exhausted_sender_sequence_refuses_before_a_new_original_or_revision() {
    let until = Instant::now() + Duration::from_secs(20);
    let f = Fixture::new();
    let store_before = fs::read(&f.store_path).unwrap();
    let message = ChatMessage {
        scope: f.policy.scope,
        channel: Channel::General,
        author: Author {
            account: f.alice.public.account,
            device: f.alice.public.device,
        },
        message: [81; 16],
        sequence: u64::MAX,
        text: "last representable sequence".into(),
    };
    let signature = f.alice.device_key.sign(&hash(&message_bytes(&message)));
    let signed = SignedMessage { message, signature };
    let mut outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 0).unwrap();
    outbox
        .enqueue(RequestId::from_bytes([99; 16]), &signed)
        .unwrap();
    drop(outbox);
    let before = fs::read(&f.outbox_path).unwrap();
    assert_eq!(
        submit(&f, 1, f.known(), 37, 101, "would overflow"),
        Err(IpcError::Limit)
    );
    assert_eq!(fs::read(&f.outbox_path).unwrap(), before);
    assert_eq!(fs::read(&f.store_path).unwrap(), store_before);
    let outbox = ClientOutbox::open_existing(&f.outbox_path, &f.profile, 1).unwrap();
    assert_eq!(outbox.known_revision(), Ok(1));
    assert_eq!(outbox.entry([81; 16]).unwrap().unwrap().signed, signed);
    assert!(Instant::now() < until);
}
