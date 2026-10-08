#[path = "supplies_crash_support/fixture.rs"]
mod fixture;
#[path = "supplies_crash_support/process.rs"]
mod process;
#[path = "supplies_crash_support/protocol.rs"]
mod protocol;
use fixture::Fixture;
use nf_contract::identity::RequestId;
use nf_kernel::supplies::IssuanceOutcome;
use nf_store::supplies::ChallengeRequest;
use process::OwnedWorker;
use protocol::Cut;
use std::time::{Duration, Instant};

#[test]
fn before_issue_public_call_crash_preserves_genesis() {
    let fixture = Fixture::new(10);
    let mut store = fixture.create();
    fixture.observe(&mut store, 0, 0, &[fixture.issue.request]);
    let known = store.known_frontiers().unwrap();
    drop(store);
    fixture.descriptor(Cut::BeforeCall, known);
    let mut worker = OwnedWorker::spawn(fixture.root());
    assert_eq!(
        &worker.frame::<8>("prepared", Instant::now() + protocol::MANAGEMENT),
        protocol::PREPARED
    );
    assert_eq!(
        &worker.frame::<8>("before", Instant::now() + protocol::MANAGEMENT),
        protocol::BEFORE
    );
    assert!(!fixture.root().join("go").exists());
    assert!(
        !fixture.root().join("template").exists(),
        "no target challenge before this public cut"
    );
    worker.kill_and_reap();
    let mut store = fixture.reopen(known);
    fixture.observe(&mut store, 0, 0, &[fixture.issue.request]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&fixture.issue));
    assert_eq!(
        store.issue(&fixture.issue, proof).unwrap(),
        Some(IssuanceOutcome {
            issuance: fixture.issue.issuance,
            revision: 1
        })
    );
    fixture.observe(&mut store, 25, 1, &[fixture.issue.request]);
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = fixture.reopen(known);
    fixture.observe(&mut store, 25, 1, &[fixture.issue.request]);
}
#[test]
fn after_issue_success_without_wrapper_ack_crash_recovers_one_issuance() {
    let fixture = Fixture::new(11);
    let mut store = fixture.create();
    fixture.observe(&mut store, 0, 0, &[fixture.issue.request]);
    let known = store.known_frontiers().unwrap();
    drop(store);
    fixture.descriptor(Cut::AfterReturn, known);
    let mut worker = OwnedWorker::spawn(fixture.root());
    assert_eq!(
        &worker.frame::<8>("prepared", Instant::now() + protocol::MANAGEMENT),
        protocol::PREPARED
    );
    assert_eq!(
        &worker.frame::<8>("before", Instant::now() + protocol::MANAGEMENT),
        protocol::BEFORE
    );
    // This lower bound precedes actual child challenge creation; expiry is a management SETUP failure.
    let before_go = Instant::now();
    let conservative_until = before_go + Duration::from_secs(5);
    protocol::write_new(&fixture.root().join("go"), protocol::GO);
    let template = worker.frame::<297>("template", conservative_until);
    let signature = fixture.sign_actual_template(&template, known);
    assert!(
        Instant::now() < conservative_until,
        "conservative signing prerequisite remains live"
    );
    protocol::write_new(&fixture.root().join("signature"), &signature);
    let returned = worker.frame::<32>("returned", conservative_until);
    assert!(
        Instant::now() < conservative_until,
        "conservative result prerequisite remains live"
    );
    let original = IssuanceOutcome {
        issuance: fixture.issue.issuance,
        revision: 1,
    };
    assert_eq!(
        returned,
        protocol::returned(original),
        "marker follows the actual successful public return"
    );
    assert!(
        !fixture.root().join("ack").exists(),
        "wrapper acknowledgment is withheld"
    );
    worker.kill_and_reap();
    let mut store = fixture.reopen(known);
    fixture.observe(&mut store, 25, 1, &[fixture.issue.request]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&fixture.issue));
    assert_eq!(store.issue(&fixture.issue, proof).unwrap(), Some(original));
    let mut alias = fixture.issue;
    alias.request = RequestId::from_bytes([88; 16]);
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&alias));
    assert_eq!(store.issue(&alias, proof).unwrap(), Some(original));
    fixture.observe(&mut store, 25, 1, &[fixture.issue.request, alias.request]);
    store.compact().unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = fixture.reopen(known);
    fixture.observe(&mut store, 25, 1, &[fixture.issue.request, alias.request]);
}
#[test]
#[ignore = "Management-only child dispatcher; invoked by two owned process tests with a real descriptor, not a behavioral case"]
fn supplies_crash_worker_dispatcher() {
    let root = std::env::var_os("NF_SUPPLIES_WORKER_ROOT")
        .expect("management worker requires its actual descriptor directory");
    process::run_worker(std::path::Path::new(&root));
}
