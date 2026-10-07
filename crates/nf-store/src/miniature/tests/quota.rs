use super::quota_fixture::*;
use crate::{StoreError, miniature::*};
use nf_kernel::miniature::*;
fn attempt(
    store: &mut MiniatureStore,
    f: &mut Fixture,
    request: ChallengeRequest<'_>,
) -> ProofAttempt {
    let issued = store.issue_challenge(request).unwrap();
    ProofAttempt {
        ticket: issued.ticket,
        proof: f.signer.sign(&issued.template).unwrap(),
    }
}
fn full_frontier(
    store: &mut MiniatureStore,
    f: &mut Fixture,
) -> (Vec<MiniatureIntent>, Vec<ProofAttempt>) {
    let intents: Vec<_> = (1..=64)
        .map(|n| intent(store.world(), &f.policy.owner, n))
        .collect();
    let proofs = intents
        .iter()
        .map(|i| attempt(store, f, ChallengeRequest::Prepare(i)))
        .collect();
    (intents, proofs)
}
fn cap_at_current_pages(store: &MiniatureStore) {
    let pages: u32 = store
        .connection
        .pragma_query_value(None, "page_count", |r| r.get(0))
        .unwrap();
    store
        .connection
        .pragma_update(None, "max_page_count", pages)
        .unwrap();
    let cap: u32 = store
        .connection
        .pragma_query_value(None, "max_page_count", |r| r.get(0))
        .unwrap();
    assert_eq!(cap, pages);
}
#[test]
fn actual_sqlite_full_on_prepare_quarantines_and_reopens_exact_before_state() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let mut store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    claim(&mut store, &mut f);
    let known = store.known_frontiers().unwrap();
    let before = store.snapshot_bytes().unwrap();
    let (intents, proofs) = full_frontier(&mut store, &mut f);
    cap_at_current_pages(&store);
    assert_eq!(
        store.prepare(intents, proofs),
        Err(MiniatureStoreError::Storage(StoreError::Full))
    );
    assert_eq!(store.world().metadata().tick.0, 0);
    assert!(
        matches!(
            store.snapshot_bytes(),
            Err(MiniatureStoreError::Storage(StoreError::Quarantined))
        ),
        "actual SQLite write refusal must quarantine the owner"
    );
    drop(store);
    let restored = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(restored.snapshot_bytes().unwrap(), before);
}
#[test]
fn actual_sqlite_full_on_advance_has_no_ack_or_debit_and_recovers_exact_pending() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let mut store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    claim(&mut store, &mut f);
    let (intents, proofs) = full_frontier(&mut store, &mut f);
    store.prepare(intents, proofs).unwrap();
    let owner = f.policy.owner.clone();
    let active = attempt(
        &mut store,
        &mut f,
        ChallengeRequest::Activity {
            actor: owner.account,
            device: owner.device,
        },
    );
    store.accept_activity(active).unwrap();
    let batch = settle_miniature(
        store.world(),
        store.pending().unwrap(),
        store.authority().unwrap().context(),
    )
    .unwrap();
    let proof = attempt(&mut store, &mut f, ChallengeRequest::Commit);
    let known = store.known_frontiers().unwrap();
    let before = store.snapshot_bytes().unwrap();
    let world = encode_miniature_snapshot(store.world()).unwrap();
    cap_at_current_pages(&store);
    assert_eq!(
        store.commit(&batch, proof),
        Err(MiniatureStoreError::Storage(StoreError::Full))
    );
    assert_eq!(
        encode_miniature_snapshot(store.world()).unwrap(),
        world,
        "no tick or debit escapes a refused transaction"
    );
    assert!(
        matches!(
            store.snapshot_bytes(),
            Err(MiniatureStoreError::Storage(StoreError::Quarantined))
        ),
        "actual SQLite write refusal must quarantine the owner"
    );
    drop(store);
    let restored = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(restored.snapshot_bytes().unwrap(), before);
    assert!(restored.pending().is_some());
}
