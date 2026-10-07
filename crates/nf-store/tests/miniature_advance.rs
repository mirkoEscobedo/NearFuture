mod miniature_support;
use miniature_support::*;
use nf_kernel::miniature::*;
use nf_store::miniature::*;
fn attempt(
    store: &mut MiniatureStore,
    f: &mut Fixture,
    request: ChallengeRequest<'_>,
) -> ProofAttempt {
    let issued = store.issue_challenge(request).unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    ProofAttempt {
        ticket: issued.ticket,
        proof,
    }
}
#[test]
fn fresh_activity_and_owner_fence_allow_exactly_one_empty_tick_then_require_new_activity() {
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
    store.prepare(vec![], vec![]).unwrap();
    let request = ChallengeRequest::Activity {
        actor: f.policy.owner.account,
        device: f.policy.owner.device,
    };
    let active = attempt(&mut store, &mut f, request);
    store
        .accept_activity(active)
        .expect("a real current controller Lease-purpose proof must become live activity");
    let batch = settle_miniature(
        store.world(),
        store.pending().unwrap(),
        store.authority().unwrap().context(),
    )
    .unwrap();
    let proof = attempt(&mut store, &mut f, ChallengeRequest::Commit);
    let ack = store
        .commit(&batch, proof)
        .expect("live activity and fresh exact owner fence must atomically commit one tick");
    assert_eq!(ack.sequence().0, 1);
    assert_eq!(store.world().metadata().tick.0, 1);
    assert_eq!(store.world().metadata().provider_revision.0, 1);
    assert_eq!(store.world().component().revision().0, 0);
    store.prepare(vec![], vec![]).unwrap();
    let second = settle_miniature(
        store.world(),
        store.pending().unwrap(),
        store.authority().unwrap().context(),
    )
    .unwrap();
    let proof = attempt(&mut store, &mut f, ChallengeRequest::Commit);
    assert_eq!(
        store.commit(&second, proof),
        Err(MiniatureStoreError::NoActivity)
    );
    assert_eq!(store.world().metadata().tick.0, 1);
    let known = store.known_frontiers().unwrap();
    let bytes = store.snapshot_bytes().unwrap();
    drop(store);
    let restored = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(restored.snapshot_bytes().unwrap(), bytes);
    assert_eq!(restored.world().metadata().tick.0, 1);
}
