use super::transcript_fields::{compare, frontier, mismatch_cancel};
use super::{Scratch, attempt, claim, create, hex};
use nf_identity::model::MembershipRepository;
use nf_kernel::miniature::*;
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
pub fn all_remaining_purposes() {
    let scratch = Scratch::new();
    let (mut store, mut signer) = create(&scratch.database());
    claim(&mut store, &mut signer);
    let raw = hex(include_str!(
        "../../../../crates/nf-kernel/tests/fixtures/miniature/intent-colony.hex"
    )
    .trim());
    let intent = decode_miniature_intent(&raw).unwrap();
    let prepare = store
        .issue_challenge(ChallengeRequest::Prepare(&intent))
        .unwrap();
    compare(
        &prepare,
        "prepare",
        &store,
        3,
        0,
        Sha256::digest(&raw).into(),
    );
    store
        .prepare(vec![intent.clone()], vec![attempt(&prepare, &mut signer)])
        .unwrap();
    let initial = frontier(&store, 0);
    let commit = store.issue_challenge(ChallengeRequest::Commit).unwrap();
    compare(
        &commit,
        "commit",
        &store,
        4,
        0,
        Sha256::digest(&initial).into(),
    );
    mismatch_cancel(&mut store, &mut signer, &commit);
    let cancel = store.issue_challenge(ChallengeRequest::Cancel).unwrap();
    compare(
        &cancel,
        "cancel",
        &store,
        5,
        0,
        Sha256::digest(&initial).into(),
    );
    let proof = attempt(&cancel, &mut signer);
    let retry = ProofAttempt {
        ticket: proof.ticket,
        proof: proof.proof.clone(),
    };
    assert_eq!(
        store.claim_authority(proof),
        Err(MiniatureStoreError::WrongPurpose)
    );
    assert_eq!(
        store.cancel_pending(MiniatureCancelCause::Cancelled, retry),
        Err(MiniatureStoreError::Replay)
    );
    let scope = store.known_frontiers().unwrap().storage.scope;
    let mut member = store.load_membership(scope).unwrap().unwrap();
    member.revision = 1;
    store.commit_membership(Some(0), &member).unwrap();
    let resume = store
        .issue_challenge(ChallengeRequest::Resume(intent.request))
        .unwrap();
    compare(&resume, "resume", &store, 6, 1, Sha256::digest(&raw).into());
    store
        .resume_pending(vec![attempt(&resume, &mut signer)])
        .unwrap();
    let refreshed = frontier(&store, 1);
    assert_ne!(initial, refreshed);
    let cancel = store.issue_challenge(ChallengeRequest::Cancel).unwrap();
    compare(
        &cancel,
        "cancel",
        &store,
        7,
        1,
        Sha256::digest(&refreshed).into(),
    );
    store
        .cancel_pending(
            MiniatureCancelCause::Cancelled,
            attempt(&cancel, &mut signer),
        )
        .unwrap();
    assert_eq!(store.world().metadata().tick.0, 0);
    assert_eq!(store.world().metadata().event_sequence.0, 1);
    assert!(store.pending().is_none());
}
