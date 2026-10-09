mod taint_support;

use nf_contract::identity::RequestId;
use nf_store::registration::lease::taint::{
    AuthorityLineageId, KnownTaintFrontier, TaintCause, TaintError, TaintRecorded, TaintStore,
};
use std::{collections::BTreeMap, ffi::OsString, path::Path};
use taint_support::Fixture;

#[test]
fn authenticated_prohibited_manifest_self_report_is_durable_and_old_lease_copy_cannot_erase_it() {
    let fixture = Fixture::new();
    let (registration, known_registration, grant, known_admission) = fixture.lease();
    let old_profile = fixture.db().with_file_name("old-profile-two.sqlite");
    std::fs::copy(fixture.db(), &old_profile).unwrap();
    let policy = fixture.taint_policy();
    let genesis = KnownTaintFrontier {
        scope: fixture.registration_policy.scope,
        revision: 0,
        head: [0; 32],
        minimum_membership_revision: 1,
    };
    let mut owner = TaintStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        &policy,
        known_registration,
        known_admission,
        genesis,
    )
    .expect("explicit headless self-report owner over actual committed profile two");
    assert_eq!(owner.known_taint_frontier().unwrap(), genesis);
    let request = fixture.request(grant, known_registration, known_admission, genesis);
    assert_eq!(request.binding, registration.binding);
    assert_ne!(request.observed_manifest, policy.expected_manifest);
    let proof = fixture.attempt(&mut owner, &request);
    let recorded = owner.mark_prohibited_manifest(&request, proof).expect(
        "fresh authenticated prohibited manifest self-report must commit its first cause record",
    );
    let expected = TaintRecorded {
        original_request: RequestId::from_bytes([46; 16]),
        binding: registration.binding,
        lineage: AuthorityLineageId::from_bytes([42; 32]).unwrap(),
        revision: 1,
        cause: TaintCause::SelfReportedProhibitedManifest,
        expected_detector: [43; 32],
        expected_detector_version: 1,
        expected_manifest: [44; 32],
        observed_manifest: [45; 32],
    };
    assert_eq!(recorded, expected);
    let known_taint = owner.known_taint_frontier().unwrap();
    assert_eq!(
        (
            known_taint.scope,
            known_taint.revision,
            known_taint.minimum_membership_revision
        ),
        (fixture.registration_policy.scope, 1, 1)
    );
    assert_ne!(known_taint.head, [0; 32]);
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof).unwrap(),
        expected
    );
    assert_eq!(owner.known_taint_frontier().unwrap(), known_taint);
    drop(owner);

    let mut owner = TaintStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        &policy,
        known_registration,
        known_admission,
        known_taint,
    )
    .expect("recover actual durable self-report profile with retained taint prefix");
    assert_eq!(owner.known_taint_frontier().unwrap(), known_taint);
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof).unwrap(),
        recorded
    );
    assert_eq!(owner.known_taint_frontier().unwrap(), known_taint);
    drop(owner);

    let directory = fixture.db().parent().unwrap().to_path_buf();
    let before = directory_files(&directory);
    match TaintStore::open_existing(
        &old_profile,
        &fixture.registration_policy,
        &fixture.admission_policy,
        &policy,
        known_registration,
        known_admission,
        known_taint,
    ) {
        Ok(_) => panic!("an actual old profile-two copy cannot erase a retained taint record"),
        Err(error) => assert_eq!(error, TaintError::StaleBackup),
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
