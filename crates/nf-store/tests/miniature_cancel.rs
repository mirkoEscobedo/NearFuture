mod miniature_support;
use miniature_support::*;
use nf_identity::model::{MembershipRepository, Roles};
use nf_store::miniature::*;
#[test]
fn current_policy_owner_cancels_revoked_actor_pending_without_old_membership_equality_or_resource_debit()
 {
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
    let before = store.world().component().clone();
    let checked = store.authority().unwrap().membership_revision;
    let (new_owner, new_signer) = &mut f.other_signers[0];
    let mut next = f.membership.clone();
    next.revision += 1;
    next.devices
        .get_mut(&f.policy.owner.device)
        .unwrap()
        .revoked = true;
    next.accounts.get_mut(&new_owner.account).unwrap().roles = Roles::from_bits(5).unwrap();
    store
        .commit_membership(Some(f.membership.revision), &next)
        .unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(store.authority().unwrap().membership_revision, checked);
    let issued = store
        .issue_challenge(ChallengeRequest::Claim {
            actor: new_owner.account,
            device: new_owner.device,
        })
        .unwrap();
    let proof = new_signer.sign(&issued.template).unwrap();
    store
        .claim_authority(ProofAttempt {
            ticket: issued.ticket,
            proof,
        })
        .unwrap();
    let issued = store.issue_challenge(ChallengeRequest::Cancel).unwrap();
    let proof = new_signer.sign(&issued.template).unwrap();
    let ack = store
        .cancel_pending(
            MiniatureCancelCause::AdmissionChanged,
            ProofAttempt {
                ticket: issued.ticket,
                proof,
            },
        )
        .expect("new current owner must cancel the old-policy retained frontier atomically");
    assert_eq!(ack.sequence().0, 1);
    assert_eq!(store.world().metadata().tick.0, 0);
    assert_eq!(store.world().component(), &before);
    assert!(store.pending().is_none());
    let status = store
        .query_bound(i.request, i.actor, i.device, f.policy.scope)
        .unwrap()
        .unwrap()
        .status;
    assert!(matches!(
        status,
        MiniatureRequestStatus::Committed {
            rejection: Some(nf_kernel::miniature::MiniatureRejection::AdmissionChanged),
            ..
        }
    ));
    assert_eq!(
        store.authority().unwrap().membership_revision,
        next.revision
    );
    let known = store.known_frontiers().unwrap();
    let bytes = store.snapshot_bytes().unwrap();
    drop(store);
    assert_eq!(
        MiniatureStore::open_existing(scratch.db(), known, f.policy.auth)
            .unwrap()
            .snapshot_bytes()
            .unwrap(),
        bytes
    );
}
