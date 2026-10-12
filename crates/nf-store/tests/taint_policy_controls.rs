mod taint_support;

use nf_contract::identity::{DeviceId, HistoryId, RequestId};
use nf_identity::model::IdentityError;
use nf_store::registration::lease::taint::{
    AuthorityLineageId, KnownTaintFrontier, MarkProhibitedManifest, ProofAttempt, TaintCause,
    TaintError, TaintRecorded, TaintStore,
};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use taint_support::Fixture;

type ProfileState = (i32, i64, i64, i64, i64);
const LEASE_ONLY: ProfileState = (2, 0, 0, 0, 0);
const ONE_CAUSE: ProfileState = (3, 3, 1, 1, 1);

fn profile_state(path: &Path) -> ProfileState {
    let inspect = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let version = inspect
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    let tables = inspect
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' \
             AND name IN ('taint_meta','taint_journal','taint_lineages')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    if tables == 0 {
        return (version, 0, 0, 0, 0);
    }
    let meta = inspect
        .query_row("SELECT COUNT(*) FROM taint_meta", [], |row| row.get(0))
        .unwrap();
    let journal = inspect
        .query_row("SELECT COUNT(*) FROM taint_journal", [], |row| row.get(0))
        .unwrap();
    let lineages = inspect
        .query_row("SELECT COUNT(*) FROM taint_lineages", [], |row| row.get(0))
        .unwrap();
    (version, tables, meta, journal, lineages)
}

fn open(
    fixture: &Fixture,
    request: &MarkProhibitedManifest,
    frontier: KnownTaintFrontier,
) -> TaintStore {
    TaintStore::open_existing(
        fixture.db(),
        &fixture.registration_policy,
        &fixture.admission_policy,
        &fixture.taint_policy(),
        request.known_registration,
        request.known_admission,
        frontier,
    )
    .expect("same authenticated headless lease profile with retained frontier")
}

fn setup() -> (Fixture, TaintStore, MarkProhibitedManifest) {
    let fixture = Fixture::new();
    let (_, registration, grant, admission) = fixture.lease();
    let genesis = KnownTaintFrontier {
        scope: fixture.registration_policy.scope,
        revision: 0,
        head: [0; 32],
        minimum_membership_revision: 1,
    };
    let request = fixture.request(grant, registration, admission, genesis);
    assert_eq!(profile_state(&fixture.db()), LEASE_ONLY);
    let owner = open(&fixture, &request, genesis);
    assert_eq!(owner.known_taint_frontier().unwrap(), genesis);
    (fixture, owner, request)
}

fn assert_unchanged_and_reopens(
    fixture: &Fixture,
    owner: TaintStore,
    original: &MarkProhibitedManifest,
    frontier: KnownTaintFrontier,
    expected: ProfileState,
) {
    assert_eq!(owner.known_taint_frontier().unwrap(), frontier);
    drop(owner);
    assert_eq!(profile_state(&fixture.db()), expected);
    let owner = open(fixture, original, frontier);
    assert_eq!(owner.known_taint_frontier().unwrap(), frontier);
    drop(owner);
    assert_eq!(profile_state(&fixture.db()), expected);
}

fn first_cause(
    fixture: &Fixture,
    mut owner: TaintStore,
    request: &MarkProhibitedManifest,
) -> (TaintStore, TaintRecorded, KnownTaintFrontier) {
    let proof = fixture.attempt(&mut owner, request);
    let recorded = owner.mark_prohibited_manifest(request, proof).unwrap();
    assert_eq!(recorded.original_request, RequestId::from_bytes([46; 16]));
    assert_eq!(
        recorded.lineage,
        AuthorityLineageId::from_bytes([42; 32]).unwrap()
    );
    assert_eq!(recorded.cause, TaintCause::SelfReportedProhibitedManifest);
    assert_eq!(recorded.revision, 1);
    let frontier = owner.known_taint_frontier().unwrap();
    assert_eq!(frontier.scope, fixture.registration_policy.scope);
    assert_eq!(frontier.revision, 1);
    assert_eq!(frontier.minimum_membership_revision, 1);
    assert_ne!(frontier.head, [0; 32]);
    drop(owner);
    assert_eq!(profile_state(&fixture.db()), ONE_CAUSE);
    let owner = open(fixture, request, frontier);
    assert_eq!(owner.known_taint_frontier().unwrap(), frontier);
    (owner, recorded, frontier)
}

#[test]
fn matching_manifest_report_is_refused_without_taint_activation() {
    let (fixture, mut owner, original) = setup();
    let mut request = original;
    request.observed_manifest = request.expected_manifest;
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof),
        Err(TaintError::Policy)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn foreign_history_report_is_refused_without_taint_activation() {
    let (fixture, mut owner, original) = setup();
    let mut request = original;
    request.binding.scope.history = HistoryId::from_bytes([91; 16]);
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof),
        Err(TaintError::Scope)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn zero_authority_lineage_is_refused_without_taint_activation() {
    let (fixture, owner, original) = setup();
    assert_eq!(
        AuthorityLineageId::from_bytes([0; 32]),
        Err(TaintError::Policy)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn unsupported_detector_version_is_refused_without_taint_activation() {
    let (fixture, mut owner, original) = setup();
    let mut request = original;
    request.expected_detector_version = 2;
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof),
        Err(TaintError::Policy)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn unknown_committed_grant_is_refused_without_taint_activation() {
    let (fixture, mut owner, original) = setup();
    let mut request = original;
    request.grant.original_request = RequestId::from_bytes([92; 16]);
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof),
        Err(TaintError::UnknownGrant)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn unknown_device_cannot_obtain_a_self_report_ticket() {
    let (fixture, mut owner, original) = setup();
    let mut request = original;
    request.device = DeviceId::from_bytes([0; 16]);
    match owner.issue_taint_challenge(&request) {
        Err(error) => assert_eq!(error, TaintError::Identity(IdentityError::UnknownDevice)),
        Ok(_) => panic!("an unknown device obtained an authenticated self-report ticket"),
    }
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn altered_signature_is_refused_without_taint_activation() {
    let (fixture, mut owner, original) = setup();
    let mut proof = fixture.attempt(&mut owner, &original);
    proof.proof.signature[0] ^= 1;
    assert_eq!(
        owner.mark_prohibited_manifest(&original, proof),
        Err(TaintError::Identity(IdentityError::Signature))
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn signed_request_substitution_is_refused_without_taint_activation() {
    let (fixture, mut owner, original) = setup();
    let proof = fixture.attempt(&mut owner, &original);
    let mut substituted = original;
    substituted.observed_manifest = [93; 32];
    assert_eq!(
        owner.mark_prohibited_manifest(&substituted, proof),
        Err(TaintError::Identity(IdentityError::Frontier))
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn a_ticket_from_the_previous_owner_runtime_is_refused() {
    let (fixture, mut owner, original) = setup();
    let proof = fixture.attempt(&mut owner, &original);
    drop(owner);
    let mut owner = open(&fixture, &original, original.known_taint);
    assert_eq!(
        owner.mark_prohibited_manifest(&original, proof),
        Err(TaintError::Replay)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn a_consumed_ticket_cannot_repeat_the_recorded_cause() {
    let (fixture, mut owner, original) = setup();
    let proof = fixture.attempt(&mut owner, &original);
    let copied = ProofAttempt {
        ticket: proof.ticket,
        proof: proof.proof.clone(),
    };
    let recorded = owner.mark_prohibited_manifest(&original, proof).unwrap();
    assert_eq!(recorded.revision, 1);
    let frontier = owner.known_taint_frontier().unwrap();
    assert_eq!(frontier.revision, 1);
    assert_eq!(
        owner.mark_prohibited_manifest(&original, copied),
        Err(TaintError::Replay)
    );
    // Keep the consuming runtime alive through Replay; inspect only after closing it.
    assert_eq!(owner.known_taint_frontier().unwrap(), frontier);
    drop(owner);
    assert_eq!(profile_state(&fixture.db()), ONE_CAUSE);
    let owner = open(&fixture, &original, frontier);
    assert_unchanged_and_reopens(&fixture, owner, &original, frontier, ONE_CAUSE);
}

#[test]
fn an_unavailable_retained_taint_revision_is_refused() {
    let (fixture, mut owner, original) = setup();
    let mut request = original;
    request.known_taint.revision = 1;
    request.known_taint.head = [94; 32];
    let proof = fixture.attempt(&mut owner, &request);
    assert_eq!(
        owner.mark_prohibited_manifest(&request, proof),
        Err(TaintError::StaleBackup)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, original.known_taint, LEASE_ONLY);
}

#[test]
fn original_request_with_different_manifest_cannot_replace_its_cause() {
    let (fixture, owner, original) = setup();
    let (mut owner, _, frontier) = first_cause(&fixture, owner, &original);
    let mut conflicting = original;
    conflicting.observed_manifest = [95; 32];
    let proof = fixture.attempt(&mut owner, &conflicting);
    assert_eq!(
        owner.mark_prohibited_manifest(&conflicting, proof),
        Err(TaintError::Conflict)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, frontier, ONE_CAUSE);
}

#[test]
fn a_new_request_cannot_replace_an_existing_lineage_cause() {
    let (fixture, owner, original) = setup();
    let (mut owner, _, frontier) = first_cause(&fixture, owner, &original);
    let mut conflicting = original;
    conflicting.request = RequestId::from_bytes([96; 16]);
    let proof = fixture.attempt(&mut owner, &conflicting);
    assert_eq!(
        owner.mark_prohibited_manifest(&conflicting, proof),
        Err(TaintError::Conflict)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, frontier, ONE_CAUSE);
}

#[test]
fn later_matching_manifest_cannot_clear_a_recorded_prohibited_manifest() {
    let (fixture, owner, original) = setup();
    let (mut owner, _, frontier) = first_cause(&fixture, owner, &original);
    let mut clean = original;
    clean.observed_manifest = clean.expected_manifest;
    let proof = fixture.attempt(&mut owner, &clean);
    assert_eq!(
        owner.mark_prohibited_manifest(&clean, proof),
        Err(TaintError::Policy)
    );
    assert_unchanged_and_reopens(&fixture, owner, &original, frontier, ONE_CAUSE);
}
