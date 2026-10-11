mod chat_support;
use chat_support::Fixture;
use nf_contract::identity::RequestId;
use nf_store::chat::{ChatStore, ReceiverClock, codec::encode_signed_message};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

fn retain(root: &std::path::Path, name: &str, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
struct FixedReceiverClock;
impl ReceiverClock for FixedReceiverClock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

#[test]
fn every_new_alias_spends_the_same_account_bucket_and_exact_retry_is_free() {
    let until = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new();
    let root = PathBuf::from(
        std::env::var_os("NF_CHAT_RATE_ADMISSION_EVIDENCE_DIR")
            .expect("NF_CHAT_RATE_ADMISSION_EVIDENCE_DIR"),
    );
    assert!(root.is_absolute());
    assert!(!root.starts_with(&fixture.scratch.0));
    fs::create_dir(&root).unwrap(); // Exact external owned leaf must be absent at admission.
    let database = fixture.database();
    let signed = fixture.signed_message();
    retain(
        &root,
        "signed.original.bin",
        &encode_signed_message(&signed).unwrap(),
    );
    let clock = Arc::new(FixedReceiverClock);
    let mut store = ChatStore::create_with_clock(
        &database,
        &fixture.policy,
        &fixture.membership,
        clock.clone(),
    )
    .unwrap();
    let burst_start = Instant::now();
    let first = fixture.post(&mut store, 100, &signed).unwrap();
    for request in 101..=109 {
        assert_eq!(fixture.post(&mut store, request, &signed), Ok(first));
    }
    assert_eq!(store.known_frontiers().unwrap().revision, 1);
    let page = fixture.permitted_history(&mut store, 90).unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].signed, signed);
    // One original plus nine aliases have spent ten; the next new alias must be transiently refused.
    // Capture originals before the intended assertion so a genuine RED keeps both SQL snapshots.
    retain(
        &root,
        "before-refusal.sqlite",
        &fs::read(&database).unwrap(),
    );
    let refused = fixture.post(&mut store, 110, &signed);
    let burst_elapsed = burst_start.elapsed();
    retain(
        &root,
        "burst-elapsed-ns.original.txt",
        burst_elapsed.as_nanos().to_string().as_bytes(),
    );
    retain(&root, "after-refusal.sqlite", &fs::read(&database).unwrap());
    // Wall elapsed is diagnostic only; the trusted receiver clock remains exactly zero.
    assert_eq!(clock.now(), Duration::ZERO);
    assert!(
        refused.is_err(),
        "eleventh fresh alias unexpectedly accepted: {refused:?}"
    );
    assert_eq!(
        fs::read(root.join("before-refusal.sqlite")).unwrap(),
        fs::read(root.join("after-refusal.sqlite")).unwrap()
    );
    assert_eq!(fixture.post(&mut store, 109, &signed), Ok(first));
    assert_eq!(fixture.post(&mut store, 100, &signed), Ok(first));
    assert_eq!(store.known_frontiers().unwrap().revision, 1);
    retain(
        &root,
        "after-exact-retries.sqlite",
        &fs::read(&database).unwrap(),
    );
    assert_eq!(
        fs::read(root.join("after-refusal.sqlite")).unwrap(),
        fs::read(root.join("after-exact-retries.sqlite")).unwrap()
    );
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut reopened =
        ChatStore::open_existing_with_clock(&database, &fixture.policy, known, clock).unwrap();
    assert_eq!(fixture.post(&mut reopened, 109, &signed), Ok(first));
    assert_eq!(fixture.post(&mut reopened, 100, &signed), Ok(first));
    assert!(fixture.post(&mut reopened, 110, &signed).is_err());
    retain(&root, "after-reopen.sqlite", &fs::read(&database).unwrap());
    assert_eq!(
        fs::read(root.join("after-refusal.sqlite")).unwrap(),
        fs::read(root.join("after-reopen.sqlite")).unwrap()
    );
    assert_eq!(reopened.known_frontiers().unwrap().revision, 1);
    assert_eq!(first.original_request, RequestId::from_bytes([100; 16]));
    assert!(Instant::now() < until);
}
