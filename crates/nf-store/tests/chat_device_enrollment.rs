mod chat_support;
use chat_support::Fixture;
use nf_contract::identity::{DeviceId, RequestId};
use nf_identity::{
    enrollment::enrollment_digest,
    keys::{SecretSeed, random_id},
    model::{DeviceEnrollment, DeviceEnrollmentProof, DeviceProof},
    signing::device_digest,
};
use nf_store::chat::{
    Author, ChallengeRequest, ChatReceipt, ChatStore, ChatStoreError, ProofAttempt, ReceiverClock,
    SignedMessage, codec,
};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

struct FixedClock;
impl ReceiverClock for FixedClock {
    fn now(&self) -> Duration {
        Duration::ZERO
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
    assert_eq!(fs::read(root.join(name)).unwrap(), bytes);
}
fn post_enrolled(
    store: &mut ChatStore,
    change: &DeviceEnrollment,
    key: &SecretSeed,
    signed: &SignedMessage,
) -> Result<ChatReceipt, ChatStoreError> {
    let request = RequestId::from_bytes([120; 16]);
    let issued = store.issue_challenge(
        ChallengeRequest::Post {
            request,
            message: signed,
        },
        &change.new_peer,
    )?;
    let mut proof = DeviceProof {
        scope: change.scope,
        account: change.account,
        device: change.new_device,
        frontier: issued.membership_revision,
        peer: change.new_peer.clone(),
        challenge: issued.challenge,
        signature: [0; 64],
    };
    proof.signature = key.sign(&device_digest(&proof).unwrap());
    store.post(
        request,
        signed,
        ProofAttempt {
            ticket: issued.ticket,
            proof: &proof,
            peer: &change.new_peer,
        },
    )
}

#[test]
fn genuine_enrolled_device_shares_account_exhaustion_and_preserves_original() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let parent = std::path::PathBuf::from(
        std::env::var_os("NF_CHAT_DEVICE_ENROLLMENT_EVIDENCE_DIR")
            .expect("ROOT_OWNED_EVIDENCE_PARENT_REQUIRED"),
    );
    assert!(parent.is_absolute());
    let parent = parent.canonicalize().unwrap();
    let scratch = fixture.scratch.0.canonicalize().unwrap();
    assert!(!parent.starts_with(&scratch) && !scratch.starts_with(&parent));
    let evidence = parent.join("existing-account-second-device");
    fs::create_dir(&evidence).expect("FRESH_EVIDENCE_LEAF_REQUIRED");
    let mut store = ChatStore::create_with_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        Arc::new(FixedClock),
    )
    .unwrap();
    let original = fixture.signed_message();
    let receipt = fixture.post(&mut store, 100, &original).unwrap();
    // Nine fresh aliases cost admissions without duplicating the original displayed message.
    for id in 101..=109 {
        assert_eq!(fixture.post(&mut store, id, &original), Ok(receipt));
    }
    assert!(
        Instant::now() < until,
        "ENROLLMENT_CASE_DEADLINE_AFTER_DEPLETION"
    );
    let before_members = store.current_membership().unwrap();
    let before_frontiers = store.known_frontiers().unwrap();
    let before_history = fixture.permitted_history(&mut store, 90).unwrap();
    assert_eq!(before_history.entries.len(), 1);
    let before_rate = store.receiver_rate_policy().unwrap();
    retain(
        &evidence,
        "before-enrollment.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    retain(
        &evidence,
        "membership.before.bin",
        &nf_identity::codec::encode_state(&before_members).unwrap(),
    );
    retain(
        &evidence,
        "original.signed.bin",
        &codec::encode_signed_message(&original).unwrap(),
    );
    let new_key = SecretSeed::generate().unwrap();
    let change = DeviceEnrollment {
        scope: fixture.policy.scope,
        account: fixture.alice.public.account,
        frontier: before_members.revision,
        new_device: DeviceId::from_bytes(random_id().unwrap()),
        new_key: new_key.public_key(),
        new_peer: b"fixture-chat-added-device".to_vec(),
    };
    assert!(!before_members.devices.contains_key(&change.new_device));
    let digest = enrollment_digest(&change).unwrap();
    let proof = DeviceEnrollmentProof {
        account_signature: fixture.alice.account_key.sign(&digest),
        new_device_signature: new_key.sign(&digest),
    };
    let next = store.enroll_device(&change, &proof).unwrap();
    assert_eq!(next.scope, before_members.scope);
    assert_eq!(next.owner, before_members.owner);
    assert_eq!(next.accounts, before_members.accounts);
    assert_eq!(next.consumed, before_members.consumed);
    assert_eq!(next.revision, before_members.revision + 1);
    assert_eq!(next.devices.len(), before_members.devices.len() + 1);
    for (id, old) in &before_members.devices {
        assert_eq!(next.devices.get(id), Some(old));
    }
    let added = next.devices.get(&change.new_device).unwrap();
    assert_eq!(added.account, change.account);
    assert_eq!(added.key, change.new_key);
    assert_eq!(added.peer, change.new_peer);
    assert!(!added.revoked);
    assert_eq!(store.current_membership().unwrap(), next);
    let frontiers = store.known_frontiers().unwrap();
    assert_eq!(frontiers.scope, before_frontiers.scope);
    assert_eq!(frontiers.revision, before_frontiers.revision);
    assert_eq!(
        frontiers.membership_revision,
        before_frontiers.membership_revision + 1
    );
    assert_eq!(store.receiver_rate_policy().unwrap(), before_rate);
    let after_enrollment = fs::read(fixture.database()).unwrap();
    retain(&evidence, "after-enrollment.sqlite", &after_enrollment);
    retain(
        &evidence,
        "membership.after.bin",
        &nf_identity::codec::encode_state(&next).unwrap(),
    );
    retain(&evidence, "enrollment.digest.bin", &digest);
    retain(
        &evidence,
        "enrollment.account-signature.bin",
        &proof.account_signature,
    );
    retain(
        &evidence,
        "enrollment.device-signature.bin",
        &proof.new_device_signature,
    );
    let mut fresh = original.clone();
    fresh.message.author = Author {
        account: change.account,
        device: change.new_device,
    };
    fresh.message.message = [82; 16];
    fresh.message.sequence = 1;
    fresh.signature = new_key.sign(&codec::message_digest(&fresh.message).unwrap());
    assert_eq!(
        post_enrolled(&mut store, &change, &new_key, &fresh),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    assert_eq!(fs::read(fixture.database()).unwrap(), after_enrollment);
    // Fresh authentication at the new membership frontier preserves the old exact receipt.
    assert_eq!(fixture.post(&mut store, 100, &original), Ok(receipt));
    let history = fixture.permitted_history(&mut store, 91).unwrap();
    assert_eq!(history, before_history);
    assert_eq!(store.known_frontiers().unwrap(), frontiers);
    assert_eq!(store.receiver_rate_policy().unwrap(), before_rate);
    assert_eq!(fs::read(fixture.database()).unwrap(), after_enrollment);
    retain(
        &evidence,
        "refused-new-device.signed.bin",
        &codec::encode_signed_message(&fresh).unwrap(),
    );
    retain(
        &evidence,
        "after-refusal-and-original-retry.sqlite",
        &after_enrollment,
    );
    assert!(
        Instant::now() < until,
        "ENROLLMENT_CASE_DEADLINE_AT_QUALIFICATION"
    );
}
