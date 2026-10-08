mod registration_support;

use nf_contract::identity::{BranchId, HistoryId, RequestId};
use nf_identity::model::MembershipRepository;
use nf_store::registration::{
    AllowedBranch, BranchRegistrar, CampaignBinding, KnownRegistrationFrontier, ProofAttempt,
    RegisterBranch, RegisteredBranch, RegistrationError, RegistrationPolicy,
};
use registration_support::Fixture;
use rusqlite::{Connection, params, types::Value};
use std::{collections::BTreeMap, ffi::OsString, path::Path};

#[test]
fn immutable_bindings_retained_prefix_and_stale_copy_are_distinct() {
    let fixture = two_branch_fixture();
    let mut registrar = fixture.create();
    let first = fixture.request();
    let original = register(&fixture, &mut registrar, &first);
    assert_eq!(original, expected_receipt(&first, 1));
    let once = registrar.known_frontier().unwrap();
    assert_eq!((once.revision, once.minimum_membership_revision), (1, 1));
    assert_ne!(once.head, [0; 32]);

    let mut rebound = first;
    rebound.branch = BranchId::from_bytes([37; 16]);
    let proof = fixture.attempt(&mut registrar, &rebound);
    assert_eq!(
        registrar.register_branch(&rebound, proof),
        Err(RegistrationError::Conflict)
    );
    assert_eq!(registrar.known_frontier().unwrap(), once);
    let mut alias = first;
    alias.request = RequestId::from_bytes([39; 16]);
    let proof = fixture.attempt(&mut registrar, &alias);
    assert_eq!(
        registrar.register_branch(&alias, proof),
        Err(RegistrationError::Conflict)
    );
    assert_eq!(registrar.known_frontier().unwrap(), once);
    assert_eq!(register(&fixture, &mut registrar, &first), original);
    assert_eq!(registrar.known_frontier().unwrap(), once);

    drop(registrar);
    let old_copy = fixture.db().with_file_name("old-registration.sqlite");
    std::fs::copy(fixture.db(), &old_copy).unwrap();
    let mut registrar =
        BranchRegistrar::open_existing(fixture.db(), &fixture.policy, once).unwrap();
    let second = second_request(&fixture);
    let next = register(&fixture, &mut registrar, &second);
    assert_eq!(next, expected_receipt(&second, 2));
    let twice = registrar.known_frontier().unwrap();
    assert_eq!((twice.revision, twice.minimum_membership_revision), (2, 1));
    assert_ne!(twice.head, once.head);
    drop(registrar);

    // Retained revision-one head remains valid after a legitimate second registration.
    let mut registrar =
        BranchRegistrar::open_existing(fixture.db(), &fixture.policy, once).unwrap();
    assert_eq!(registrar.known_frontier().unwrap(), twice);
    assert_eq!(register(&fixture, &mut registrar, &first), original);
    assert_eq!(register(&fixture, &mut registrar, &second), next);
    assert_eq!(registrar.known_frontier().unwrap(), twice);
    drop(registrar);

    assert_open_refused_preserving_files(
        &old_copy,
        &fixture.policy,
        twice,
        RegistrationError::StaleBackup,
    );
    let mut forged = once;
    forged.head[0] ^= 1;
    assert_open_refused_preserving_files(
        &fixture.db(),
        &fixture.policy,
        forged,
        RegistrationError::Corrupt,
    );
}

#[test]
fn every_branch_cache_field_is_checked_against_the_journal() {
    for vector in [
        CacheVector::Account,
        CacheVector::OriginalRequest,
        CacheVector::Revision,
    ] {
        let fixture = two_branch_fixture();
        let mut registrar = fixture.create();
        let first = fixture.request();
        let second = second_request(&fixture);
        let membership = registrar
            .load_membership(fixture.policy.scope)
            .unwrap()
            .unwrap();
        assert_eq!(membership.revision, 1);
        let founder = membership.owner;
        assert_ne!(founder, first.account);
        assert_eq!(
            register(&fixture, &mut registrar, &first),
            expected_receipt(&first, 1)
        );
        assert_eq!(
            register(&fixture, &mut registrar, &second),
            expected_receipt(&second, 2)
        );
        let known = registrar.known_frontier().unwrap();
        assert_eq!((known.revision, known.minimum_membership_revision), (2, 1));
        drop(registrar);

        let connection = Connection::open(fixture.db()).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        let before = journal_and_meta(&connection);
        let changed = match vector {
            CacheVector::Account => connection.execute(
                "UPDATE registration_branches SET account=?1 WHERE campaign=?2 AND branch=?3",
                params![founder.as_bytes(), first.campaign.as_bytes(), first.branch.as_bytes()]),
            CacheVector::OriginalRequest => connection.execute(
                "UPDATE registration_branches SET original_request=?1 WHERE campaign=?2 AND branch=?3",
                params![second.request.as_bytes(), first.campaign.as_bytes(), first.branch.as_bytes()]),
            CacheVector::Revision => connection.execute(
                "UPDATE registration_branches SET revision=?1 WHERE campaign=?2 AND branch=?3",
                params![3u64.to_be_bytes(), first.campaign.as_bytes(), first.branch.as_bytes()]),
        }.unwrap();
        assert_eq!(changed, 1);
        assert_eq!(journal_and_meta(&connection), before);
        drop(connection);
        assert_open_refused_preserving_files(
            &fixture.db(),
            &fixture.policy,
            known,
            RegistrationError::Corrupt,
        );
    }
}

#[test]
fn signed_invalid_scope_and_policy_consume_proofs_without_claiming_request() {
    let fixture = Fixture::new();
    let mut registrar = fixture.create();
    let correct = fixture.request();
    let genesis = registrar.known_frontier().unwrap();
    assert_eq!(
        (genesis.revision, genesis.minimum_membership_revision),
        (0, 1)
    );
    assert_eq!(genesis.head, [0; 32]);
    let mut wrong_scope = correct;
    wrong_scope.scope.history = HistoryId::from_bytes([38; 16]);
    let mut wrong_policy = correct;
    wrong_policy.policy_digest[0] ^= 1;

    for (invalid, expected) in [
        (wrong_scope, RegistrationError::Scope),
        (wrong_policy, RegistrationError::Policy),
    ] {
        let before = directory_files(fixture.db().parent().unwrap());
        let proof = fixture.attempt(&mut registrar, &invalid);
        let duplicate = ProofAttempt {
            ticket: proof.ticket,
            proof: proof.proof.clone(),
        };
        assert_eq!(registrar.register_branch(&invalid, proof), Err(expected));
        assert_eq!(registrar.known_frontier().unwrap(), genesis);
        assert_eq!(
            registrar.register_branch(&invalid, duplicate),
            Err(RegistrationError::Replay)
        );
        assert_eq!(registrar.known_frontier().unwrap(), genesis);
        assert_eq!(directory_files(fixture.db().parent().unwrap()), before);
    }
    assert_eq!(
        register(&fixture, &mut registrar, &correct),
        expected_receipt(&correct, 1)
    );
    let committed = registrar.known_frontier().unwrap();
    assert_eq!(
        (committed.revision, committed.minimum_membership_revision),
        (1, 1)
    );
    assert_ne!(committed.head, genesis.head);
}

fn two_branch_fixture() -> Fixture {
    let mut fixture = Fixture::new();
    let original = fixture.policy.allowed[0];
    fixture.policy.allowed.push(AllowedBranch {
        branch: BranchId::from_bytes([37; 16]),
        ..original
    });
    fixture.policy.validate().unwrap();
    fixture
}

fn second_request(fixture: &Fixture) -> RegisterBranch {
    RegisterBranch {
        request: RequestId::from_bytes([39; 16]),
        branch: BranchId::from_bytes([37; 16]),
        ..fixture.request()
    }
}

fn register(
    fixture: &Fixture,
    registrar: &mut BranchRegistrar,
    request: &RegisterBranch,
) -> RegisteredBranch {
    let proof = fixture.attempt(registrar, request);
    registrar
        .register_branch(request, proof)
        .expect("actual signed registration")
}

fn expected_receipt(request: &RegisterBranch, revision: u64) -> RegisteredBranch {
    RegisteredBranch {
        binding: CampaignBinding {
            scope: request.scope,
            campaign: request.campaign,
            branch: request.branch,
            account: request.account,
        },
        original_request: request.request,
        revision,
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

fn assert_open_refused_preserving_files(
    path: &Path,
    policy: &RegistrationPolicy,
    known: KnownRegistrationFrontier,
    expected: RegistrationError,
) {
    let directory = path.parent().unwrap();
    let before = directory_files(directory);
    match BranchRegistrar::open_existing(path, policy, known) {
        Ok(_) => panic!("invalid registration history must be refused"),
        Err(error) => assert_eq!(error, expected),
    }
    assert_eq!(directory_files(directory), before);
}

#[derive(Clone, Copy)]
enum CacheVector {
    Account,
    OriginalRequest,
    Revision,
}

fn journal_and_meta(connection: &Connection) -> (Vec<Vec<Value>>, Vec<Vec<Value>>) {
    let journal = sql_rows(
        connection,
        "SELECT revision,request,previous,request_digest,head,body FROM registration_journal ORDER BY revision",
    );
    assert_eq!(journal.len(), 2);
    let meta = sql_rows(
        connection,
        "SELECT singleton,universe,history,revision,head,policy_digest,schema_digest,policy_body FROM registration_meta",
    );
    assert_eq!(meta.len(), 1);
    (journal, meta)
}

fn sql_rows(connection: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut statement = connection.prepare(sql).unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
