mod lease_support;

use lease_support::Fixture;
use nf_store::registration::{
    BranchRegistrar,
    lease::{AdmissionMode, KnownAdmissionFrontier, LeaseError, LeaseGranted, LeaseStore},
};
use std::{collections::BTreeMap, ffi::OsString, path::Path};

#[test]
fn registered_player_receives_one_durable_headless_lease_and_recovers_its_actual_grant() {
    let fixture = Fixture::new();
    let (registration, known_registration) = fixture.register();
    let registrar = BranchRegistrar::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        known_registration,
    )
    .expect("reopen actual registered profile one");
    assert_eq!(registrar.known_frontier().unwrap(), known_registration);
    drop(registrar);
    let old_profile = fixture.db().with_file_name("old-profile-one.sqlite");
    std::fs::copy(fixture.db(), &old_profile).unwrap();
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
    .expect("explicit trusted headless lease owner");
    assert_eq!(owner.known_admission_frontier().unwrap(), genesis);
    let request = fixture.request(registration, known_registration, genesis);
    let proof = fixture.attempt(&mut owner, &request);
    let granted = owner
        .admit(&request, proof)
        .expect("fresh authenticated admission must commit its exact first lease grant");
    let expected = LeaseGranted {
        mode: AdmissionMode::HeadlessLeaseOnly,
        registration,
        original_request: request.request,
        device: request.device,
        session: request.session,
        generation: 1,
        revision: 1,
    };
    assert_eq!(granted, expected);
    let known_admission = owner.known_admission_frontier().unwrap();
    assert_eq!(
        (
            known_admission.revision,
            known_admission.minimum_membership_revision
        ),
        (1, 1)
    );
    assert_ne!(known_admission.head, genesis.head);
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(owner.admit(&request, proof).unwrap(), expected);
    assert_eq!(owner.known_admission_frontier().unwrap(), known_admission);
    drop(owner);

    let mut owner = LeaseStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        known_registration,
        known_admission,
    )
    .expect("recover actual committed lease profile");
    assert_eq!(owner.known_admission_frontier().unwrap(), known_admission);
    let proof = fixture.attempt(&mut owner, &request);
    let recovered = owner.admit(&request, proof).unwrap();
    assert_eq!(recovered, granted);
    assert_eq!(owner.known_admission_frontier().unwrap(), known_admission);
    drop(owner);

    let directory = fixture.db().parent().unwrap().to_path_buf();
    let before = directory_files(&directory);
    match LeaseStore::open_existing(
        &old_profile,
        &fixture.registration_policy,
        &fixture.admission_policy,
        known_registration,
        known_admission,
    ) {
        Ok(_) => panic!("profile-one copy cannot erase a retained actual lease grant"),
        Err(error) => assert_eq!(error, LeaseError::StaleBackup),
    }
    assert_eq!(directory_files(&directory), before);
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
