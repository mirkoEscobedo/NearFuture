#[path = "miniature_support/flow.rs"]
mod flow;
mod miniature_support;
use miniature_support::*;
use nf_kernel::miniature::*;
use nf_store::{StoreError, miniature::*};
use nf_world::WorldAction;
#[test]
fn invalid_first_candidate_reserves_nothing_next_legal_wins_and_later_conflicts_durably() {
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
    let mut invalid = intent(store.world(), &f.policy.owner, 1);
    let legal = intent(store.world(), &f.policy.owner, 2);
    let later = intent(store.world(), &f.policy.owner, 3);
    let WorldAction::CreateColony { site, .. } = invalid.action else {
        panic!()
    };
    invalid.action = WorldAction::CreateColony {
        site,
        faction: store
            .world()
            .component()
            .factions()
            .iter()
            .find(|x| x.account != f.policy.owner.account)
            .unwrap()
            .id,
    };
    flow::prepare(
        &mut store,
        &mut f,
        vec![later.clone(), invalid.clone(), legal.clone()],
    );
    let frontier = store.pending().unwrap();
    let hold = frontier.reservation().unwrap();
    assert_eq!(hold.operation(), legal.operation);
    assert_eq!((hold.credits(), hold.supplies()), (100, 40));
    let outcomes = frontier.outcomes();
    assert_eq!(
        outcomes
            .iter()
            .find(|o| o.job == invalid.job)
            .unwrap()
            .rejection,
        Some(MiniatureRejection::InvalidReference)
    );
    assert_eq!(
        outcomes
            .iter()
            .find(|o| o.job == later.job)
            .unwrap()
            .rejection,
        Some(MiniatureRejection::Conflict)
    );
    assert_eq!(
        store
            .world()
            .component()
            .factions()
            .iter()
            .find(|x| x.account == f.policy.owner.account)
            .unwrap()
            .credits,
        200
    );
    flow::advance(&mut store, &mut f);
    let faction = store
        .world()
        .component()
        .factions()
        .iter()
        .find(|x| x.account == f.policy.owner.account)
        .unwrap();
    assert_eq!(
        (
            faction.credits,
            faction.supplies,
            faction.spent_credits,
            faction.spent_supplies
        ),
        (100, 60, 100, 40)
    );
    assert!(store.pending().is_none());
    assert_eq!(store.outbox().unwrap().len(), 3);
    let revision = store.known_frontiers().unwrap();
    for i in [&invalid, &legal, &later] {
        let bound = store
            .query_bound(i.request, i.actor, i.device, f.policy.scope)
            .unwrap()
            .unwrap();
        assert_eq!(bound.intent, *i);
        assert!(matches!(
            bound.status,
            MiniatureRequestStatus::Committed {
                sequence: nf_contract::identity::EventSeq(1),
                ..
            }
        ));
    }
    assert_eq!(
        store.known_frontiers().unwrap(),
        revision,
        "retained exact-request queries do not debit or advance"
    );
    let mut changed = legal.clone();
    changed.action = invalid.action;
    let proof = flow::attempt(&mut store, &mut f, ChallengeRequest::Prepare(&changed));
    assert!(store.prepare(vec![changed], vec![proof]).is_err());
    assert_eq!(store.known_frontiers().unwrap(), revision);
    assert_eq!(
        store.query_bound(
            legal.request,
            f.other_signers[0].0.account,
            legal.device,
            f.policy.scope
        ),
        Err(MiniatureStoreError::Storage(StoreError::RequestConflict))
    );
    let bytes = store.snapshot_bytes().unwrap();
    drop(store);
    let reopened = MiniatureStore::open_existing(scratch.db(), revision, f.policy.auth).unwrap();
    assert_eq!(reopened.snapshot_bytes().unwrap(), bytes);
}
#[test]
fn old_provider_revision_is_rejected_even_when_component_revision_is_unchanged() {
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
    let stale = intent(store.world(), &f.policy.owner, 1);
    flow::prepare(&mut store, &mut f, vec![]);
    flow::advance(&mut store, &mut f);
    assert_eq!(store.world().component().revision().0, 0);
    assert_eq!(store.world().metadata().provider_revision.0, 1);
    let current = intent(store.world(), &f.policy.owner, 2);
    flow::prepare(&mut store, &mut f, vec![current.clone(), stale.clone()]);
    assert_eq!(
        store.pending().unwrap().reservation().unwrap().operation(),
        current.operation
    );
    assert_eq!(
        store
            .pending()
            .unwrap()
            .outcomes()
            .iter()
            .find(|o| o.job == stale.job)
            .unwrap()
            .rejection,
        Some(MiniatureRejection::StaleRevision)
    );
    flow::advance(&mut store, &mut f);
    assert_eq!(store.world().metadata().tick.0, 2);
}
