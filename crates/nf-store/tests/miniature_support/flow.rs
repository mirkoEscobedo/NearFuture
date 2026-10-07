#![allow(dead_code)]
use crate::miniature_support::*;
use nf_kernel::miniature::*;
use nf_store::miniature::*;
use std::path::Path;
pub fn attempt(
    store: &mut MiniatureStore,
    f: &mut Fixture,
    request: ChallengeRequest<'_>,
) -> ProofAttempt {
    let i = store.issue_challenge(request).unwrap();
    ProofAttempt {
        ticket: i.ticket,
        proof: f.signer.sign(&i.template).unwrap(),
    }
}
pub fn prepare(store: &mut MiniatureStore, f: &mut Fixture, intents: Vec<MiniatureIntent>) {
    let proofs = intents
        .iter()
        .map(|i| attempt(store, f, ChallengeRequest::Prepare(i)))
        .collect();
    store.prepare(intents, proofs).unwrap();
}
pub fn advance(store: &mut MiniatureStore, f: &mut Fixture) {
    let owner = f.policy.owner.clone();
    let lease = attempt(
        store,
        f,
        ChallengeRequest::Activity {
            actor: owner.account,
            device: owner.device,
        },
    );
    store.accept_activity(lease).unwrap();
    let batch = settle_miniature(
        store.world(),
        store.pending().unwrap(),
        store.authority().unwrap().context(),
    )
    .unwrap();
    let proof = attempt(store, f, ChallengeRequest::Commit);
    store.commit(&batch, proof).unwrap();
}
pub fn reopen(store: MiniatureStore, f: &mut Fixture, path: &Path) -> MiniatureStore {
    let known = store.known_frontiers().unwrap();
    let bytes = store.snapshot_bytes().unwrap();
    drop(store);
    let mut restored = MiniatureStore::open_existing(path, known, f.policy.auth).unwrap();
    assert_eq!(
        restored.snapshot_bytes().unwrap(),
        bytes,
        "closed durable journal replay must be byte exact before fresh authority"
    );
    claim(&mut restored, f);
    if restored.pending().is_some() {
        let requests: Vec<_> = restored
            .pending()
            .unwrap()
            .intents()
            .iter()
            .map(|i| i.request)
            .collect();
        let proofs = requests
            .into_iter()
            .map(|r| attempt(&mut restored, f, ChallengeRequest::Resume(r)))
            .collect();
        restored.resume_pending(proofs).unwrap();
    }
    restored
}
