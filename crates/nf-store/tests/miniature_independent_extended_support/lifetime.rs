use super::{Scratch, attempt, create};
use nf_contract::identity::*;
use nf_store::{StoreError, miniature::*};
use std::time::Duration;
fn request() -> ChallengeRequest<'static> {
    ChallengeRequest::Claim {
        actor: AccountId::from_bytes([4; 16]),
        device: DeviceId::from_bytes([33; 16]),
    }
}
pub fn configuration_and_capacity() {
    for millis in [0, 999, 30_001, u64::MAX] {
        assert_eq!(
            AuthConfig::new(Duration::from_millis(millis)),
            Err(MiniatureStoreError::Storage(StoreError::Limit))
        );
    }
    for millis in [1000, 30_000] {
        assert!(AuthConfig::new(Duration::from_millis(millis)).is_ok());
    }
    let scratch = Scratch::new();
    let (mut store, mut signer) = create(&scratch.database());
    let mut issued = Vec::new();
    for _ in 0..64 {
        issued.push(store.issue_challenge(request()).unwrap());
    }
    assert!(matches!(
        store.issue_challenge(request()),
        Err(MiniatureStoreError::Storage(StoreError::Backpressure))
    ));
    let first = issued.remove(0);
    let good = attempt(&first, &mut signer);
    let mut forged = good.proof.clone();
    forged.peer = vec![99];
    let forged = signer.sign(&forged).unwrap();
    assert!(
        store
            .claim_authority(ProofAttempt {
                ticket: good.ticket,
                proof: forged
            })
            .is_err()
    );
    assert_eq!(
        store.claim_authority(good),
        Err(MiniatureStoreError::Replay)
    );
    assert!(store.authority().is_none());
    assert_eq!(store.known_frontiers().unwrap().storage.store_revision, 0);
    store.issue_challenge(request()).unwrap();
    assert!(matches!(
        store.issue_challenge(request()),
        Err(MiniatureStoreError::Storage(StoreError::Backpressure))
    ));
}
pub fn expired_before_transaction_is_consumed() {
    let scratch = Scratch::new();
    let (store, mut signer) = create(&scratch.database());
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut store = MiniatureStore::open_existing(
        scratch.database(),
        known,
        AuthConfig::new(Duration::from_secs(1)).unwrap(),
    )
    .unwrap();
    let issued = store.issue_challenge(request()).unwrap();
    let proof = attempt(&issued, &mut signer);
    let ticket = proof.ticket;
    let retry = proof.proof.clone();
    std::thread::sleep(Duration::from_millis(1100));
    assert_eq!(
        store.claim_authority(proof),
        Err(MiniatureStoreError::Expired)
    );
    assert_eq!(
        store.claim_authority(ProofAttempt {
            ticket,
            proof: retry
        }),
        Err(MiniatureStoreError::Replay)
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    assert!(store.authority().is_none());
    let next = store.issue_challenge(request()).unwrap();
    assert_ne!(next.challenge_preimage(), issued.challenge_preimage());
    store.claim_authority(attempt(&next, &mut signer)).unwrap();
    assert_eq!(store.authority().unwrap().term.0, 1);
}
