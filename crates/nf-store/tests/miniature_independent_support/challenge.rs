use super::{Scratch, fixture, vector};
use nf_contract::identity::*;
use nf_store::miniature::*;
use sha2::{Digest, Sha256};
fn transcript(issued: &IssuedChallenge, name: &str, term: u64, sequence: u64) {
    let actual = issued.challenge_preimage();
    let mut expected = vector("auth", name);
    expected[80..88].copy_from_slice(&term.to_le_bytes());
    let proposed = if name == "claim" { term + 1 } else { term };
    expected[88..96].copy_from_slice(&proposed.to_le_bytes());
    // Only OS-fresh opaque values are read back. All owner/context fields remain independent.
    expected[96..104].copy_from_slice(&actual[96..104]);
    expected[112..120].copy_from_slice(&0u64.to_le_bytes());
    expected[184..192].copy_from_slice(&sequence.to_le_bytes());
    expected[192..224].copy_from_slice(&actual[192..224]);
    assert_eq!(actual.as_slice(), expected);
    assert_ne!(&actual[96..104], &[0; 8]);
    assert_ne!(&actual[192..224], &[0; 32]);
    assert_eq!(
        issued.template.challenge,
        <[u8; 32]>::from(Sha256::digest(actual))
    );
}
pub fn issued_transcripts_and_first_attempt_are_bound() {
    let scratch = Scratch::new();
    let (mut store, mut signer) = fixture::create(&scratch.database());
    let owner = AccountId::from_bytes([4; 16]);
    let device = DeviceId::from_bytes([33; 16]);
    let claim = ChallengeRequest::Claim {
        actor: owner,
        device,
    };
    let issued = store.issue_challenge(claim).unwrap();
    transcript(&issued, "claim", 0, 2);
    let proof = signer.sign(&issued.template).unwrap();
    let ticket = issued.ticket;
    store
        .claim_authority(ProofAttempt {
            ticket,
            proof: proof.clone(),
        })
        .unwrap();
    assert_eq!(
        store.claim_authority(ProofAttempt { ticket, proof }),
        Err(MiniatureStoreError::Replay)
    );
    let authority = store.authority().unwrap();
    let issued = store
        .issue_challenge(ChallengeRequest::Activity {
            actor: owner,
            device,
        })
        .unwrap();
    transcript(&issued, "lease", 1, 3);
    assert_eq!(
        u64::from_le_bytes(issued.challenge_preimage()[96..104].try_into().unwrap()),
        authority.session.0
    );
    let proof = signer.sign(&issued.template).unwrap();
    let ticket = issued.ticket;
    assert_eq!(
        store.claim_authority(ProofAttempt {
            ticket,
            proof: proof.clone()
        }),
        Err(MiniatureStoreError::WrongPurpose)
    );
    assert_eq!(
        store.accept_activity(ProofAttempt { ticket, proof }),
        Err(MiniatureStoreError::Replay)
    );
    let issued = store
        .issue_challenge(ChallengeRequest::Activity {
            actor: owner,
            device,
        })
        .unwrap();
    transcript(&issued, "lease", 1, 4);
    let proof = signer.sign(&issued.template).unwrap();
    store
        .accept_activity(ProofAttempt {
            ticket: issued.ticket,
            proof,
        })
        .unwrap();
    assert_eq!(store.world().metadata().tick.0, 0);
    assert_eq!(store.known_frontiers().unwrap().storage.store_revision, 1);
}
