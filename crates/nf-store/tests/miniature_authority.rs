mod miniature_support;
use miniature_support::*;
use nf_store::miniature::*;
fn signed(f: &mut Fixture, issued: IssuedChallenge) -> ProofAttempt {
    let proof = f.signer.sign(&issued.template).unwrap();
    ProofAttempt {
        ticket: issued.ticket,
        proof,
    }
}
#[test]
fn fresh_signed_claim_is_durable_and_reopen_requires_greater_term() {
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
    let issued = store
        .issue_challenge(ChallengeRequest::Claim {
            actor: f.policy.owner.account,
            device: f.policy.owner.device,
        })
        .unwrap();
    let accepted = store
        .claim_authority(signed(&mut f, issued))
        .expect("a fresh current-player candidate proof must durably claim");
    assert_eq!(accepted.revision(), 1);
    let authority = store.authority().unwrap();
    assert_eq!(authority.term.0, 1);
    assert_ne!(authority.session.0, 0);
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut reopened = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert!(matches!(
        reopened.issue_challenge(ChallengeRequest::Commit),
        Err(MiniatureStoreError::Unclaimed)
    ));
    let issued = reopened
        .issue_challenge(ChallengeRequest::Claim {
            actor: f.policy.owner.account,
            device: f.policy.owner.device,
        })
        .unwrap();
    reopened.claim_authority(signed(&mut f, issued)).unwrap();
    let next = reopened.authority().unwrap();
    assert_eq!(next.term.0, 2);
    assert_ne!(next.session, authority.session);
    assert_eq!(reopened.world().metadata().tick.0, 0);
}

#[test]
fn invalid_signature_consumes_ticket_and_policy_revocation_clears_old_proofs() {
    use nf_identity::model::MembershipRepository;
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
    let request = ChallengeRequest::Claim {
        actor: f.policy.owner.account,
        device: f.policy.owner.device,
    };
    let issued = store.issue_challenge(request).unwrap();
    let good = f.signer.sign(&issued.template).unwrap();
    let mut bad = good.clone();
    bad.signature[0] ^= 1;
    assert!(
        store
            .claim_authority(ProofAttempt {
                ticket: issued.ticket,
                proof: bad
            })
            .is_err()
    );
    assert_eq!(
        store.claim_authority(ProofAttempt {
            ticket: issued.ticket,
            proof: good
        }),
        Err(MiniatureStoreError::Replay)
    );
    assert!(store.authority().is_none());
    let issued = store.issue_challenge(request).unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    let mut revoked = f.membership.clone();
    revoked.revision += 1;
    revoked
        .devices
        .get_mut(&f.policy.owner.device)
        .unwrap()
        .revoked = true;
    store
        .commit_membership(Some(f.membership.revision), &revoked)
        .unwrap();
    assert_eq!(
        store.claim_authority(ProofAttempt {
            ticket: issued.ticket,
            proof
        }),
        Err(MiniatureStoreError::Replay)
    );
    assert!(store.issue_challenge(request).is_err());
    assert!(store.authority().is_none());
}
#[test]
fn final_precommit_deadline_expiry_rolls_back_claim_and_consumes_its_ticket() {
    use std::time::Duration;
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    f.policy.auth = AuthConfig::new(Duration::from_secs(1)).unwrap();
    let mut store = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    let issued = store
        .issue_challenge(ChallengeRequest::Claim {
            actor: f.policy.owner.account,
            device: f.policy.owner.device,
        })
        .unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    let ticket = issued.ticket;
    assert_eq!(
        store.claim_authority_with_hook(
            ProofAttempt {
                ticket,
                proof: proof.clone()
            },
            &mut |boundary| {
                if boundary == nf_store::Boundary::BeforeCommit {
                    std::thread::sleep(Duration::from_millis(1100));
                }
                Ok(())
            }
        ),
        Err(MiniatureStoreError::Expired)
    );
    assert_eq!(store.known_frontiers().unwrap().storage.store_revision, 0);
    assert!(store.authority().is_none());
    assert_eq!(
        store.claim_authority(ProofAttempt { ticket, proof }),
        Err(MiniatureStoreError::Replay)
    );
    let known = store.known_frontiers().unwrap();
    drop(store);
    assert!(
        MiniatureStore::open_existing(scratch.db(), known, f.policy.auth)
            .unwrap()
            .authority()
            .is_none()
    );
}
#[test]
fn private_registry_survives_no_reuse_after_successful_claim_or_restart() {
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
    let request = ChallengeRequest::Claim {
        actor: f.policy.owner.account,
        device: f.policy.owner.device,
    };
    let first = store.issue_challenge(request).unwrap();
    let stale = store.issue_challenge(request).unwrap();
    let stale_proof = f.signer.sign(&stale.template).unwrap();
    store.claim_authority(signed(&mut f, first)).unwrap();
    assert_eq!(
        store.claim_authority(ProofAttempt {
            ticket: stale.ticket,
            proof: stale_proof.clone()
        }),
        Err(MiniatureStoreError::Replay)
    );
    let issued = store.issue_challenge(request).unwrap();
    let proof = f.signer.sign(&issued.template).unwrap();
    let known = store.known_frontiers().unwrap();
    drop(store);
    let mut reopened = MiniatureStore::open_existing(scratch.db(), known, f.policy.auth).unwrap();
    assert_eq!(
        reopened.claim_authority(ProofAttempt {
            ticket: issued.ticket,
            proof
        }),
        Err(MiniatureStoreError::Replay)
    );
}
