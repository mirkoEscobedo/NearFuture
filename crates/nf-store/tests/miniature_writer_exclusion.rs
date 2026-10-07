mod miniature_support;
#[path = "miniature_support/process.rs"]
mod process_support;
use miniature_support::*;
use nf_contract::identity::*;
use nf_store::{KnownFrontiers, StoreError, miniature::*};
use std::{process::Command, time::Duration};
#[test]
fn miniature_second_writer_worker() {
    let Some(path) = std::env::var_os("NF_MINIATURE_LOCK_DATABASE") else {
        return;
    };
    let known = MiniatureKnownFrontiers {
        storage: KnownFrontiers::genesis(
            UniverseId::from_bytes([1; 16]),
            HistoryId::from_bytes([2; 16]),
        ),
        minimum_authority_term: AuthorityTerm(0),
    };
    let result = MiniatureStore::open_existing(path, known, AuthConfig::default());
    assert!(
        matches!(result, Err(MiniatureStoreError::Storage(StoreError::Busy))),
        "a live physical SQLite owner must exclude another writer"
    );
    std::fs::write(
        std::env::var_os("NF_MINIATURE_LOCK_RESULT").unwrap(),
        b"writer-refused",
    )
    .unwrap();
}
#[test]
fn actual_second_process_is_excluded_without_changing_the_owned_state() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    let known = store.known_frontiers().unwrap();
    let bytes = store.snapshot_bytes().unwrap();
    let result = scratch.0.join("second.result");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "miniature_second_writer_worker",
            "--test-threads=1",
        ])
        .env("NF_MINIATURE_LOCK_DATABASE", scratch.db())
        .env("NF_MINIATURE_LOCK_RESULT", &result);
    let mut child = process_support::OwnedChild::spawn(&mut command);
    assert!(child.wait(Duration::from_secs(10)).success());
    assert_eq!(
        process_support::read_bounded(&result, 32).unwrap(),
        b"writer-refused"
    );
    assert_eq!(store.snapshot_bytes().unwrap(), bytes);
    assert_eq!(store.known_frontiers().unwrap(), known);
    drop(store);
    let restored = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(restored.snapshot_bytes().unwrap(), bytes);
}
