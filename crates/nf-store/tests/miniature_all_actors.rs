#[path = "miniature_support/flow.rs"]
mod flow;
mod miniature_support;
use miniature_support::*;
use nf_store::miniature::*;
use nf_world::WorldAction;
#[test]
fn restarted_pending_requires_fresh_proofs_from_every_original_actor_and_preserves_intents() {
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
    let first = intent(store.world(), &f.policy.owner, 1);
    let public = f.other_signers[0].0.clone();
    let mut second = intent(store.world(), &public, 2);
    let world = store.world().component();
    let actor = world
        .factions()
        .iter()
        .find(|x| x.account == public.account)
        .unwrap()
        .id;
    let other = world
        .factions()
        .iter()
        .find(|x| x.account == f.policy.owner.account)
        .unwrap()
        .id;
    second.action = WorldAction::SetRelationship {
        faction: actor,
        other,
        score: 100,
    };
    let a = flow::attempt(&mut store, &mut f, ChallengeRequest::Prepare(&first));
    let issued = store
        .issue_challenge(ChallengeRequest::Prepare(&second))
        .unwrap();
    let b = ProofAttempt {
        ticket: issued.ticket,
        proof: f.other_signers[0].1.sign(&issued.template).unwrap(),
    };
    store
        .prepare(vec![first.clone(), second.clone()], vec![a, b])
        .unwrap();
    let known = store.known_frontiers().unwrap();
    let original = store.pending().unwrap().intents().to_vec();
    drop(store);
    let mut restored = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    claim(&mut restored, &mut f);
    let before = restored.snapshot_bytes().unwrap();
    let a = flow::attempt(
        &mut restored,
        &mut f,
        ChallengeRequest::Resume(first.request),
    );
    assert_eq!(
        restored.resume_pending(vec![a]),
        Err(MiniatureStoreError::Unauthorized)
    );
    assert_eq!(restored.snapshot_bytes().unwrap(), before);
    let a = flow::attempt(
        &mut restored,
        &mut f,
        ChallengeRequest::Resume(first.request),
    );
    let issued = restored
        .issue_challenge(ChallengeRequest::Resume(second.request))
        .unwrap();
    let b = ProofAttempt {
        ticket: issued.ticket,
        proof: f.other_signers[0].1.sign(&issued.template).unwrap(),
    };
    restored.resume_pending(vec![a, b]).unwrap();
    assert_eq!(restored.pending().unwrap().intents(), original);
    flow::advance(&mut restored, &mut f);
    assert_eq!(restored.world().outcomes().len(), 2);
}
