mod miniature_support;
use miniature_support::*;
use nf_kernel::miniature::*;
use nf_store::miniature::*;
#[test]
fn prepare_derives_the_exact_positive_hold_and_retains_original_binding_on_restart() {
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
    let intent = intent(store.world(), &f.policy.owner, 40);
    let issued = store
        .issue_challenge(ChallengeRequest::Prepare(&intent))
        .unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    store
        .prepare(
            vec![intent.clone()],
            vec![ProofAttempt {
                ticket: issued.ticket,
                proof,
            }],
        )
        .expect("current economic proof must durably prepare its pure derived hold");
    let hold = store.pending().unwrap().reservation().unwrap();
    let expected = admit_miniature(
        store.world(),
        vec![intent.clone()],
        store.authority().unwrap().context(),
        f.membership.revision,
    )
    .unwrap();
    assert_eq!(hold, expected.reservation().unwrap());
    assert!(hold.credits() > 0);
    let record = store
        .query_bound(intent.request, intent.actor, intent.device, f.policy.scope)
        .unwrap()
        .unwrap();
    assert_eq!(record.intent, intent);
    assert!(matches!(
        record.status,
        MiniatureRequestStatus::Pending { .. }
    ));
    let frontier = encode_miniature_frontier(store.pending().unwrap()).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let reopened = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(
        encode_miniature_frontier(reopened.pending().unwrap()).unwrap(),
        frontier
    );
    assert_eq!(
        reopened
            .query_bound(intent.request, intent.actor, intent.device, f.policy.scope)
            .unwrap(),
        Some(record)
    );
    assert_eq!(reopened.world().metadata().tick.0, 0);
}
