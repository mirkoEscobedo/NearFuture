mod chat_support;
use chat_support::Fixture;
use nf_contract::identity::RequestId;
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, PublicIdentity},
    signing::device_digest,
};
use nf_store::chat::{
    ChallengeRequest, ChatRatePolicy, ChatReceipt, ChatStore, ChatStoreError, ProofAttempt,
    ReceiverClock, SignedMessage,
};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct Clock {
    nanos: AtomicU64,
    reads: AtomicUsize,
}
impl Clock {
    fn set(&self, nanos: u64) {
        self.nanos.store(nanos, Ordering::SeqCst);
    }
    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}
impl ReceiverClock for Clock {
    fn now(&self) -> Duration {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Duration::from_nanos(self.nanos.load(Ordering::SeqCst))
    }
}
fn root(fixture: &Fixture, name: &str) -> PathBuf {
    let parent = PathBuf::from(
        std::env::var_os("NF_CHAT_RATE_GREEN_CONTROLS_EVIDENCE_DIR")
            .expect("NF_CHAT_RATE_GREEN_CONTROLS_EVIDENCE_DIR"),
    );
    assert!(parent.is_absolute() && !parent.starts_with(&fixture.scratch.0));
    let root = parent.join(name);
    fs::create_dir(&root).unwrap();
    root
}
fn retain(root: &std::path::Path, name: &str, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn post_as(
    store: &mut ChatStore,
    identity: &PublicIdentity,
    key: &SecretSeed,
    request: u8,
    signed: &SignedMessage,
) -> Result<ChatReceipt, ChatStoreError> {
    let request = RequestId::from_bytes([request; 16]);
    let issued = store.issue_challenge(
        ChallengeRequest::Post {
            request,
            message: signed,
        },
        &identity.peer,
    )?;
    let mut proof = DeviceProof {
        scope: signed.message.scope,
        account: identity.account,
        device: identity.device,
        frontier: issued.membership_revision,
        peer: identity.peer.clone(),
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
            peer: &identity.peer,
        },
    )
}
fn signed_as(
    fixture: &Fixture,
    identity: &PublicIdentity,
    key: &SecretSeed,
    id: u8,
    sequence: u64,
) -> SignedMessage {
    let mut signed = fixture.signed_message();
    signed.message.author = nf_store::chat::Author {
        account: identity.account,
        device: identity.device,
    };
    signed.message.message = [id; 16];
    signed.message.sequence = sequence;
    signed.signature = key.sign(&nf_store::chat::codec::message_digest(&signed.message).unwrap());
    signed
}
#[test]
fn default_ten_burst_fractional_refill_and_alias_retry_are_authentic() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "default-ten");
    let clock = Arc::new(Clock::default());
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        ChatRatePolicy::default(),
        clock.clone(),
    )
    .unwrap();
    let signed = fixture.signed_message();
    retain(
        &evidence,
        "signed.original.bin",
        &nf_store::chat::codec::encode_signed_message(&signed).unwrap(),
    );
    let first = fixture.post(&mut store, 100, &signed).unwrap();
    for id in 101..=109 {
        assert_eq!(fixture.post(&mut store, id, &signed), Ok(first));
    }
    let before = fs::read(fixture.database()).unwrap();
    retain(&evidence, "before-refusal.sqlite", &before);
    assert_eq!(
        fixture.post(&mut store, 110, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    clock.set(1_999_999_999);
    assert_eq!(
        fixture.post(&mut store, 110, &signed),
        Err(ChatStoreError::RateLimited { retry_after_ms: 1 })
    );
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    assert_eq!(fixture.post(&mut store, 109, &signed), Ok(first));
    assert_eq!(fixture.post(&mut store, 100, &signed), Ok(first));
    clock.set(2_000_000_000);
    assert_eq!(fixture.post(&mut store, 110, &signed), Ok(first));
    assert_eq!(store.known_frontiers().unwrap().revision, 1);
    assert_eq!(
        fixture
            .permitted_history(&mut store, 99)
            .unwrap()
            .entries
            .len(),
        1
    );
    retain(
        &evidence,
        "after-refill.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}
#[test]
fn exact_retry_and_current_auth_conflicts_do_not_touch_empty_bucket() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "auth-retry");
    let clock = Arc::new(Clock::default());
    let rate = ChatRatePolicy::new(2, 2_000).unwrap();
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        rate,
        clock.clone(),
    )
    .unwrap();
    let signed = fixture.signed_message();
    let first = fixture.post(&mut store, 100, &signed).unwrap();
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(first));
    let before = fs::read(fixture.database()).unwrap();
    retain(&evidence, "before.sqlite", &before);
    let reads = clock.reads();
    assert_eq!(fixture.post(&mut store, 100, &signed), Ok(first));
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(first));
    assert_eq!(clock.reads(), reads);
    let mut conflicting = signed.clone();
    conflicting.message.text = "different binding".into();
    conflicting.signature = fixture
        .alice
        .device_key
        .sign(&nf_store::chat::codec::message_digest(&conflicting.message).unwrap());
    assert_eq!(
        fixture.post(&mut store, 101, &conflicting),
        Err(ChatStoreError::Conflict)
    );
    assert_eq!(clock.reads(), reads);
    let mut forged = signed.clone();
    forged.signature[0] ^= 1;
    assert!(fixture.post(&mut store, 102, &forged).is_err());
    assert_eq!(clock.reads(), reads);
    assert_eq!(
        fixture.post(&mut store, 102, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    retain(
        &evidence,
        "after.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}
#[test]
fn reopen_preserves_committed_fraction_without_downtime_or_startup_tokens() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "reopen-fraction");
    let clock = Arc::new(Clock::default());
    let rate = ChatRatePolicy::new(2, 2_000).unwrap();
    let signed = fixture.signed_message();
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        rate,
        clock.clone(),
    )
    .unwrap();
    let first = fixture.post(&mut store, 100, &signed).unwrap();
    assert_eq!(fixture.post(&mut store, 101, &signed), Ok(first));
    clock.set(2_500_000_000);
    assert_eq!(fixture.post(&mut store, 102, &signed), Ok(first));
    let known = store.known_frontiers().unwrap();
    let before = fs::read(fixture.database()).unwrap();
    retain(&evidence, "before-reopen.sqlite", &before);
    drop(store);
    clock.set(100_000_000_000);
    let mut store = ChatStore::open_existing_with_clock(
        fixture.database(),
        &fixture.policy,
        known,
        clock.clone(),
    )
    .unwrap();
    assert_eq!(
        fixture.post(&mut store, 103, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 1_500
        })
    );
    assert_eq!(fixture.post(&mut store, 102, &signed), Ok(first));
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    clock.set(101_499_999_999);
    assert_eq!(
        fixture.post(&mut store, 103, &signed),
        Err(ChatStoreError::RateLimited { retry_after_ms: 1 })
    );
    clock.set(101_500_000_000);
    assert_eq!(fixture.post(&mut store, 103, &signed), Ok(first));
    retain(
        &evidence,
        "after.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}
#[test]
fn policy_reload_validates_cas_preserves_baselines_and_grants_no_bonus() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "policy-reload");
    let clock = Arc::new(Clock::default());
    let old = ChatRatePolicy::new(2, 2_000).unwrap();
    let signed = fixture.signed_message();
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        old,
        clock.clone(),
    )
    .unwrap();
    fixture.post(&mut store, 100, &signed).unwrap();
    fixture.post(&mut store, 101, &signed).unwrap();
    let before = fs::read(fixture.database()).unwrap();
    let reads = clock.reads();
    clock.set(1_000_000_000);
    assert_eq!(store.reload_receiver_rate_policy(0, old), Ok(0));
    assert_eq!(clock.reads(), reads);
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    let next = ChatRatePolicy::new(10, 1_000).unwrap();
    assert_eq!(store.reload_receiver_rate_policy(0, next), Ok(1));
    assert_eq!(store.receiver_rate_policy(), Ok((next, 1)));
    assert_eq!(
        fixture.post(&mut store, 102, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 500
        })
    );
    let changed = fs::read(fixture.database()).unwrap();
    assert_eq!(
        store.reload_receiver_rate_policy(0, old),
        Err(ChatStoreError::Conflict)
    );
    assert_eq!(fs::read(fixture.database()).unwrap(), changed);
    clock.set(1_500_000_000);
    assert!(fixture.post(&mut store, 102, &signed).is_ok());
    retain(&evidence, "before.sqlite", &before);
    retain(
        &evidence,
        "after.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}

#[test]
fn authenticated_new_account_starts_full_without_refilling_existing_account() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "new-account");
    let clock = Arc::new(Clock::default());
    let rate = ChatRatePolicy::new(2, 2_000).unwrap();
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        rate,
        clock.clone(),
    )
    .unwrap();
    let alice = fixture.signed_message();
    fixture.post(&mut store, 100, &alice).unwrap();
    fixture.post(&mut store, 101, &alice).unwrap();
    let (public, account_key, device_key) =
        nf_identity::keys::generate_identity(b"rate-new-account".to_vec()).unwrap();
    let invitation = nf_identity::model::Invitation {
        scope: fixture.policy.scope,
        id: [31; 16],
        issuer: fixture.alice.public.account,
        recipient: public.clone(),
        roles: nf_identity::model::Roles::PLAYER,
        expires_at: 100,
        issued_revision: fixture.membership.revision,
        reusable: false,
    };
    let proof =
        nf_identity::signing::admission_proof(&invitation, &account_key, &device_key).unwrap();
    let signed =
        nf_identity::signing::sign_invitation(invitation, &fixture.alice.account_key).unwrap();
    store.admit_player(&signed, &proof, 1).unwrap();
    assert_eq!(
        fixture.post(&mut store, 102, &alice),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    let message = signed_as(&fixture, &public, &device_key, 83, 1);
    let first = post_as(&mut store, &public, &device_key, 120, &message).unwrap();
    assert_eq!(
        post_as(&mut store, &public, &device_key, 121, &message),
        Ok(first)
    );
    assert_eq!(
        post_as(&mut store, &public, &device_key, 122, &message),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    assert_eq!(store.known_frontiers().unwrap().revision, 2);
    retain(
        &evidence,
        "invitation.signature.original.bin",
        &signed.signature,
    );
    retain(
        &evidence,
        "account-admission.signature.original.bin",
        &proof.account_signature,
    );
    retain(
        &evidence,
        "device-admission.signature.original.bin",
        &proof.device_signature,
    );
    retain(
        &evidence,
        "new-account.signed.original.bin",
        &nf_store::chat::codec::encode_signed_message(&message).unwrap(),
    );
    retain(
        &evidence,
        "after.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}

#[test]
fn one_exclusive_owner_prevents_sibling_baseline_duplication() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "exclusive-owner");
    let clock = Arc::new(Clock::default());
    let rate = ChatRatePolicy::new(2, 2_000).unwrap();
    let signed = fixture.signed_message();
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        rate,
        clock.clone(),
    )
    .unwrap();
    let first = fixture.post(&mut store, 100, &signed).unwrap();
    fixture.post(&mut store, 101, &signed).unwrap();
    let known = store.known_frontiers().unwrap();
    let before = fs::read(fixture.database()).unwrap();
    retain(&evidence, "before-sibling.sqlite", &before);
    let sibling_clock = Arc::new(Clock::default());
    sibling_clock.set(100_000_000_000);
    let sibling = ChatStore::open_existing_with_clock(
        fixture.database(),
        &fixture.policy,
        known,
        sibling_clock,
    );
    assert!(
        sibling.is_err(),
        "a second owner could acquire an independent baseline"
    );
    drop(sibling);
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    assert_eq!(
        fixture.post(&mut store, 102, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    assert_eq!(fixture.post(&mut store, 100, &signed), Ok(first));
    drop(store);
    clock.set(100_000_000_000);
    let mut reopened = ChatStore::open_existing_with_clock(
        fixture.database(),
        &fixture.policy,
        known,
        clock.clone(),
    )
    .unwrap();
    assert_eq!(
        fixture.post(&mut reopened, 102, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    retain(
        &evidence,
        "after-release-reopen.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}
#[test]
fn capacity_decrease_and_slower_refill_preserve_fraction_across_reopen() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "reload-decrease");
    let clock = Arc::new(Clock::default());
    let old = ChatRatePolicy::new(4, 2_000).unwrap();
    let signed = fixture.signed_message();
    let mut store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        old,
        clock.clone(),
    )
    .unwrap();
    let first = fixture.post(&mut store, 100, &signed).unwrap();
    for id in 101..=103 {
        fixture.post(&mut store, id, &signed).unwrap();
    }
    clock.set(1_000_000_000);
    let next = ChatRatePolicy::new(2, 4_000).unwrap();
    assert_eq!(store.reload_receiver_rate_policy(0, next), Ok(1));
    assert_eq!(
        fixture.post(&mut store, 104, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    let known = store.known_frontiers().unwrap();
    let before = fs::read(fixture.database()).unwrap();
    retain(&evidence, "before-reopen.sqlite", &before);
    drop(store);
    clock.set(100_000_000_000);
    let mut store = ChatStore::open_existing_with_clock(
        fixture.database(),
        &fixture.policy,
        known,
        clock.clone(),
    )
    .unwrap();
    assert_eq!(store.receiver_rate_policy(), Ok((next, 1)));
    let reads = clock.reads();
    assert_eq!(fixture.post(&mut store, 100, &signed), Ok(first));
    assert_eq!(clock.reads(), reads);
    assert_eq!(
        fixture.post(&mut store, 104, &signed),
        Err(ChatStoreError::RateLimited {
            retry_after_ms: 2_000
        })
    );
    assert_eq!(fs::read(fixture.database()).unwrap(), before);
    clock.set(101_999_999_999);
    assert_eq!(
        fixture.post(&mut store, 104, &signed),
        Err(ChatStoreError::RateLimited { retry_after_ms: 1 })
    );
    clock.set(102_000_000_000);
    assert_eq!(fixture.post(&mut store, 104, &signed), Ok(first));
    retain(
        &evidence,
        "after-refill.sqlite",
        &fs::read(fixture.database()).unwrap(),
    );
    assert!(Instant::now() < until);
}

#[test]
fn corrupted_or_missing_committed_bucket_refuses_open_without_full_fallback() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let evidence = root(&fixture, "corrupt-bucket");
    let clock = Arc::new(Clock::default());
    let store = ChatStore::create_with_rate_policy_and_clock(
        fixture.database(),
        &fixture.policy,
        &fixture.membership,
        ChatRatePolicy::default(),
        clock.clone(),
    )
    .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let genuine = fs::read(fixture.database()).unwrap();
    retain(&evidence, "genuine.sqlite", &genuine);
    // Mutate only this owned disposable fixture; retained original files are read-only.
    let connection = rusqlite::Connection::open(fixture.database()).unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE chat_rate_accounts SET fractional_ns=?1 WHERE account=?2",
                rusqlite::params![
                    [255u8; 8].as_slice(),
                    fixture.alice.public.account.as_bytes()
                ]
            )
            .unwrap(),
        1
    );
    drop(connection);
    let corrupt = fs::read(fixture.database()).unwrap();
    retain(&evidence, "corrupted-fraction.sqlite", &corrupt);
    assert!(matches!(
        ChatStore::open_existing_with_clock(
            fixture.database(),
            &fixture.policy,
            known,
            clock.clone()
        ),
        Err(ChatStoreError::Corrupt)
    ));
    assert_eq!(fs::read(fixture.database()).unwrap(), corrupt);
    fs::write(fixture.database(), &genuine).unwrap();
    let connection = rusqlite::Connection::open(fixture.database()).unwrap();
    assert_eq!(
        connection
            .execute(
                "DELETE FROM chat_rate_accounts WHERE account=?1",
                [fixture.alice.public.account.as_bytes()]
            )
            .unwrap(),
        1
    );
    drop(connection);
    let missing = fs::read(fixture.database()).unwrap();
    retain(&evidence, "missing-bucket.sqlite", &missing);
    assert!(matches!(
        ChatStore::open_existing_with_clock(fixture.database(), &fixture.policy, known, clock),
        Err(ChatStoreError::Corrupt)
    ));
    assert_eq!(fs::read(fixture.database()).unwrap(), missing);
    assert!(Instant::now() < until);
}
