mod lease_support;

use lease_support::Fixture;
use nf_contract::identity::RequestId;
use nf_store::registration::{
    KnownRegistrationFrontier,
    lease::{
        AdmissionMode, AdmitLease, ClientSessionId, KnownAdmissionFrontier, LeaseError,
        LeaseGranted, LeaseStore, ProofAttempt,
    },
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::Path,
    sync::{Arc, Barrier, Mutex},
    thread,
};

#[test]
fn immutable_lease_request_and_active_branch_reject_rebinding_without_writes() {
    let fixture = Fixture::new();
    let (registration, known_registration) = fixture.register();
    let genesis = admission_genesis(&fixture);
    let mut owner = open_owner(&fixture, known_registration, genesis);
    let original = fixture.request(registration, known_registration, genesis);
    let proof = fixture.attempt(&mut owner, &original);
    let grant = owner.admit(&original, proof).unwrap();
    assert_eq!(grant, expected_grant(&original));
    let committed = owner.known_admission_frontier().unwrap();
    assert_eq!(
        (committed.revision, committed.minimum_membership_revision),
        (1, 1)
    );
    assert_ne!(committed.head, genesis.head);
    let before = directory_files(fixture.db().parent().unwrap());

    let proof = fixture.attempt(&mut owner, &original);
    assert_eq!(owner.admit(&original, proof).unwrap(), grant);
    assert_eq!(owner.known_admission_frontier().unwrap(), committed);
    assert_eq!(directory_files(fixture.db().parent().unwrap()), before);

    let mut changed_session = original;
    changed_session.session = ClientSessionId::from_bytes([42; 16]).unwrap();
    let mut changed_prefix = original;
    changed_prefix.known_admission = committed;
    let mut new_request_same_session = original;
    new_request_same_session.request = RequestId::from_bytes([43; 16]);
    let mut new_request_other_session = original;
    new_request_other_session.request = RequestId::from_bytes([44; 16]);
    new_request_other_session.session = ClientSessionId::from_bytes([45; 16]).unwrap();
    for conflicting in [
        changed_session,
        changed_prefix,
        new_request_same_session,
        new_request_other_session,
    ] {
        let proof = fixture.attempt(&mut owner, &conflicting);
        let consumed = copy_attempt(&proof);
        assert_eq!(owner.admit(&conflicting, proof), Err(LeaseError::Conflict));
        assert_eq!(owner.known_admission_frontier().unwrap(), committed);
        assert_eq!(directory_files(fixture.db().parent().unwrap()), before);
        assert_eq!(owner.admit(&conflicting, consumed), Err(LeaseError::Replay));
        assert_eq!(owner.known_admission_frontier().unwrap(), committed);
        assert_eq!(directory_files(fixture.db().parent().unwrap()), before);
    }
    let proof = fixture.attempt(&mut owner, &original);
    assert_eq!(owner.admit(&original, proof).unwrap(), grant);
    assert_eq!(owner.known_admission_frontier().unwrap(), committed);
    assert_eq!(directory_files(fixture.db().parent().unwrap()), before);
}

#[test]
fn competing_signed_admissions_commit_one_grant_through_one_canonical_owner() {
    let fixture = Fixture::new();
    let (registration, known_registration) = fixture.register();
    let genesis = admission_genesis(&fixture);
    let mut owner = open_owner(&fixture, known_registration, genesis);
    let first = fixture.request(registration, known_registration, genesis);
    let mut second = first;
    second.request = RequestId::from_bytes([46; 16]);
    second.session = ClientSessionId::from_bytes([47; 16]).unwrap();
    assert_ne!(
        (first.request, first.session),
        (second.request, second.session)
    );
    // Both actual signed proofs exist before either admission commits.
    let first_proof = fixture.attempt(&mut owner, &first);
    let second_proof = fixture.attempt(&mut owner, &second);
    assert_eq!(owner.known_admission_frontier().unwrap(), genesis);
    let owner = Arc::new(Mutex::new(owner));
    let start = Arc::new(Barrier::new(3));
    let handles: Vec<_> = [(first, first_proof), (second, second_proof)]
        .into_iter()
        .map(|(request, proof)| {
            let owner = Arc::clone(&owner);
            let start = Arc::clone(&start);
            thread::spawn(move || {
                let consumed = copy_attempt(&proof);
                start.wait();
                let result = owner.lock().unwrap().admit(&request, proof);
                (request, consumed, result)
            })
        })
        .collect();
    start.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|(_, _, result)| result.is_ok())
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|(_, _, result)| *result == Err(LeaseError::Conflict))
            .count(),
        1
    );
    let winner = outcomes
        .iter()
        .find_map(|(request, _, result)| result.as_ref().ok().map(|grant| (*request, *grant)))
        .unwrap();
    assert_eq!(winner.1, expected_grant(&winner.0));
    let mut owner = match Arc::try_unwrap(owner) {
        Ok(owner) => owner.into_inner().unwrap(),
        Err(_) => panic!("both admission threads must release the canonical owner"),
    };
    let committed = owner.known_admission_frontier().unwrap();
    assert_eq!(
        (committed.revision, committed.minimum_membership_revision),
        (1, 1)
    );
    assert_ne!(committed.head, genesis.head);
    let before = directory_files(fixture.db().parent().unwrap());
    for (request, consumed, result) in outcomes {
        if result == Err(LeaseError::Conflict) {
            assert_eq!(owner.admit(&request, consumed), Err(LeaseError::Replay));
            assert_eq!(owner.known_admission_frontier().unwrap(), committed);
            assert_eq!(directory_files(fixture.db().parent().unwrap()), before);
        }
    }
    let proof = fixture.attempt(&mut owner, &winner.0);
    assert_eq!(owner.admit(&winner.0, proof).unwrap(), winner.1);
    assert_eq!(owner.known_admission_frontier().unwrap(), committed);
    assert_eq!(directory_files(fixture.db().parent().unwrap()), before);
    drop(owner);
    let mut owner = open_owner(&fixture, known_registration, committed);
    assert_eq!(owner.known_admission_frontier().unwrap(), committed);
    let before_reopened = directory_files(fixture.db().parent().unwrap());
    let proof = fixture.attempt(&mut owner, &winner.0);
    assert_eq!(owner.admit(&winner.0, proof).unwrap(), winner.1);
    assert_eq!(owner.known_admission_frontier().unwrap(), committed);
    assert_eq!(
        directory_files(fixture.db().parent().unwrap()),
        before_reopened
    );
    // This is one canonical SQLite authority, not cloned-node or native fencing.
}

fn admission_genesis(fixture: &Fixture) -> KnownAdmissionFrontier {
    KnownAdmissionFrontier {
        scope: fixture.registration_policy.scope,
        revision: 0,
        head: [0; 32],
        minimum_membership_revision: 1,
    }
}
fn open_owner(
    fixture: &Fixture,
    registration: KnownRegistrationFrontier,
    admission: KnownAdmissionFrontier,
) -> LeaseStore {
    LeaseStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        registration,
        admission,
    )
    .expect("actual explicit headless canonical owner")
}
fn expected_grant(request: &AdmitLease) -> LeaseGranted {
    LeaseGranted {
        mode: AdmissionMode::HeadlessLeaseOnly,
        registration: request.registration,
        original_request: request.request,
        device: request.device,
        session: request.session,
        generation: 1,
        revision: 1,
    }
}
fn copy_attempt(attempt: &ProofAttempt) -> ProofAttempt {
    ProofAttempt {
        ticket: attempt.ticket,
        proof: attempt.proof.clone(),
    }
}
fn directory_files(directory: &Path) -> BTreeMap<OsString, Vec<u8>> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(
                entry.file_type().unwrap().is_file(),
                "owned fixture contains only files"
            );
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect()
}
