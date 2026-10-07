mod miniature_support;
use miniature_support::*;
use nf_identity::model::MembershipRepository;
use nf_store::miniature::*;

#[test]
fn changed_policy_reopen_retains_paused_frontier_then_fresh_resume_preserves_intent_and_input() {
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
    let i = intent(store.world(), &f.policy.owner, 40);
    let issued = store
        .issue_challenge(ChallengeRequest::Prepare(&i))
        .unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    store
        .prepare(
            vec![i.clone()],
            vec![ProofAttempt {
                ticket: issued.ticket,
                proof,
            }],
        )
        .unwrap();
    let before = store.pending().unwrap().clone();
    let mut next = f.membership.clone();
    next.revision += 1;
    next.consumed.insert([99; 16]);
    store
        .commit_membership(Some(f.membership.revision), &next)
        .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut reopened = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(reopened.pending(), Some(&before));
    claim(&mut reopened, &mut f);
    let issued = reopened
        .issue_challenge(ChallengeRequest::Resume(i.request))
        .unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    reopened
        .resume_pending(vec![ProofAttempt {
            ticket: issued.ticket,
            proof,
        }])
        .expect("fresh complete current policy proofs must resume exact retained intent");
    let after = reopened.pending().unwrap();
    assert_eq!(after.intents(), before.intents());
    assert_eq!(after.input_hash(), before.input_hash());
    assert_eq!(after.reservation(), before.reservation());
    assert_eq!(after.membership_revision(), next.revision);
    assert_ne!(after.authority(), before.authority());
    let known = reopened.known_frontiers().unwrap();
    drop(reopened);
    let final_state = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(final_state.pending().unwrap().intents(), &[i]);
}
