mod lease_revocation_support;

use lease_revocation_support::Fixture;
use nf_identity::{
    model::{IdentityError, MembershipRepository, Roles},
    persistence::revoke_persisted,
};
use nf_store::registration::lease::{
    AdmissionMode, KnownAdmissionFrontier, LeaseError, LeaseGranted, LeaseStore, ProofAttempt,
};
use rusqlite::{Connection, types::Value};
use std::{collections::BTreeMap, ffi::OsString, path::Path};

#[test]
fn persisted_self_device_revocation_fences_an_issued_lease_proof_at_member_two() {
    let fixture = Fixture::new();
    let (registration, known_registration) = fixture.register();
    let genesis = KnownAdmissionFrontier {
        scope: fixture.registration_policy.scope,
        revision: 0,
        head: [0; 32],
        minimum_membership_revision: 1,
    };
    let mut owner = LeaseStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        known_registration,
        genesis,
    )
    .unwrap();
    let request = fixture.request(registration, known_registration, genesis);
    let proof = fixture.attempt(&mut owner, &request);
    let grant = owner.admit(&request, proof).unwrap();
    let expected_grant = LeaseGranted {
        mode: AdmissionMode::HeadlessLeaseOnly,
        registration,
        original_request: request.request,
        device: request.device,
        session: request.session,
        generation: 1,
        revision: 1,
    };
    assert_eq!(grant, expected_grant);
    let once = owner.known_admission_frontier().unwrap();
    assert_eq!((once.revision, once.minimum_membership_revision), (1, 1));
    assert_ne!(once.head, genesis.head);
    drop(owner);
    let old_copy = fixture.db().with_file_name("old-member-one.sqlite");
    std::fs::copy(fixture.db(), &old_copy).unwrap();
    let history_before = immutable_history(&fixture.db());
    let mut owner = LeaseStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        known_registration,
        once,
    )
    .unwrap();
    let member_one = owner
        .load_membership(request.binding.scope)
        .unwrap()
        .unwrap();
    assert_eq!(member_one.revision, 1);
    assert_eq!(
        member_one.accounts[&request.binding.account].roles,
        Roles::PLAYER
    );
    assert!(!member_one.devices[&request.device].revoked);
    let old_proof = fixture.attempt(&mut owner, &request);
    assert_eq!(old_proof.proof.frontier, 1);
    let consumed = ProofAttempt {
        ticket: old_proof.ticket,
        proof: old_proof.proof.clone(),
    };
    let (change, signature) = fixture.signed_self_revocation(1);
    assert_eq!(
        (change.issuer, change.device, change.frontier),
        (request.binding.account, request.device, 1)
    );
    let revoked = revoke_persisted(&mut owner, request.binding.scope, &change, &signature).expect(
        "actual account-signed self-device revocation must persist membership revision two",
    );
    assert_eq!(revoked.revision, 2);
    let mut expected_member = member_one;
    expected_member.revision = 2;
    expected_member
        .devices
        .get_mut(&request.device)
        .unwrap()
        .revoked = true;
    assert!(
        revoked == expected_member,
        "only the signed device flag and membership frontier change"
    );
    assert!(
        owner.load_membership(request.binding.scope).unwrap() == Some(expected_member.clone()),
        "same canonical authority must expose the actual committed membership two"
    );
    let twice = owner.known_admission_frontier().unwrap();
    assert_eq!(
        (
            twice.revision,
            twice.head,
            twice.minimum_membership_revision
        ),
        (1, once.head, 2)
    );
    let unchanged = directory_files(fixture.db().parent().unwrap());
    assert_eq!(
        owner.admit(&request, old_proof),
        Err(LeaseError::Identity(IdentityError::Frontier))
    );
    assert_eq!(owner.admit(&request, consumed), Err(LeaseError::Replay));
    assert_revoked_challenge(&mut owner, &request);
    assert_eq!(owner.known_admission_frontier().unwrap(), twice);
    assert!(
        directory_files(fixture.db().parent().unwrap()) == unchanged,
        "refused old proof, consumed ticket and revoked challenge must preserve files"
    );
    drop(owner);
    assert!(
        immutable_history(&fixture.db()) == history_before,
        "membership revocation cannot rewrite registration, lease journal or active grant"
    );

    let before_refusal = directory_files(fixture.db().parent().unwrap());
    match LeaseStore::open_existing(
        &old_copy,
        &fixture.registration_policy,
        &fixture.admission_policy,
        known_registration,
        twice,
    ) {
        Ok(_) => panic!("closed member-one copy cannot erase retained actual revocation"),
        Err(error) => assert_eq!(error, LeaseError::StaleBackup),
    }
    assert!(
        directory_files(fixture.db().parent().unwrap()) == before_refusal,
        "old membership refusal preserves database bytes and sidecar names"
    );
    let mut owner = LeaseStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        known_registration,
        twice,
    )
    .unwrap();
    assert!(
        owner.load_membership(request.binding.scope).unwrap() == Some(expected_member),
        "valid reopen retains actual revoked membership"
    );
    assert_eq!(owner.known_admission_frontier().unwrap(), twice);
    let before_denial = directory_files(fixture.db().parent().unwrap());
    assert_revoked_challenge(&mut owner, &request);
    assert_eq!(owner.known_admission_frontier().unwrap(), twice);
    assert!(
        directory_files(fixture.db().parent().unwrap()) == before_denial,
        "revoked challenge refusal preserves files within the reopened owner"
    );
    drop(owner);
    assert!(
        immutable_history(&fixture.db()) == history_before,
        "recovery must retain the same original durable grant and registration"
    );
}

fn assert_revoked_challenge(
    owner: &mut LeaseStore,
    request: &nf_store::registration::lease::AdmitLease,
) {
    match owner.issue_admission_challenge(request) {
        Ok(_) => panic!("revoked device cannot obtain a fresh lease proof challenge"),
        Err(error) => assert_eq!(error, LeaseError::Identity(IdentityError::Revoked)),
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
fn immutable_history(path: &Path) -> BTreeMap<&'static str, Vec<Vec<Value>>> {
    // Actual snapshots require the canonical owner to be closed; no SQL state is forged.
    let connection = Connection::open(path).unwrap();
    [
        (
            "registration_meta",
            "SELECT * FROM registration_meta ORDER BY singleton",
        ),
        (
            "registration_journal",
            "SELECT * FROM registration_journal ORDER BY revision",
        ),
        (
            "registration_branches",
            "SELECT * FROM registration_branches ORDER BY campaign,branch",
        ),
        (
            "admission_meta",
            "SELECT * FROM admission_meta ORDER BY singleton",
        ),
        (
            "admission_journal",
            "SELECT * FROM admission_journal ORDER BY revision",
        ),
        (
            "admission_active",
            "SELECT * FROM admission_active ORDER BY campaign,branch",
        ),
    ]
    .into_iter()
    .map(|(name, sql)| {
        let mut statement = connection.prepare(sql).unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
            .unwrap()
            .collect::<rusqlite::Result<Vec<Vec<Value>>>>()
            .unwrap();
        assert_eq!(
            rows.len(),
            1,
            "actual first committed registration and lease each have one row"
        );
        (name, rows)
    })
    .collect()
}
