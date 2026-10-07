mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{Reserve, policy_digest};
use nf_store::supplies::{
    ChallengeRequest, KnownSuppliesFrontiers, SuppliesStore, SuppliesStoreError,
};
use supplies_support::{Fixture, Scratch};

fn retained() -> (Scratch, Fixture, KnownSuppliesFrontiers) {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let mut store =
        SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let issue = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issue));
    assert_eq!(
        store
            .issue(&issue, proof)
            .unwrap()
            .expect("signed grant")
            .revision,
        1
    );
    let own = fixture.query();
    let reserve = Reserve {
        request: RequestId::from_bytes([44; 16]),
        reservation: OperationId::from_bytes([45; 16]),
        actor: own.actor,
        device: own.device,
        owner: own.owner,
        universe: own.universe,
        history: own.history,
        policy: policy_digest(&fixture.policy).unwrap(),
        content: own.content,
        origin: own.origin,
        amount: 8,
    };
    let proof = fixture.attempt(&mut store, ChallengeRequest::Reserve(&reserve));
    assert_eq!(
        store
            .reserve(&reserve, proof)
            .unwrap()
            .expect("signed reserve")
            .revision,
        2
    );
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    let balance = store.balance(&own, proof).unwrap();
    assert_eq!(
        (
            balance.available,
            balance.reserved,
            balance.minted,
            balance.burned,
            balance.pending,
            balance.externalized
        ),
        (17, 8, 25, 0, 0, 0)
    );
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 2);
    drop(store);
    // The identical revision2 database must open before any corruption is introduced.
    let mut store = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known).unwrap();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Balance(&own));
    assert_eq!(store.balance(&own, proof).unwrap(), balance);
    assert_eq!(store.known_frontiers().unwrap(), known);
    drop(store);
    (scratch, fixture, known)
}
fn refused_unchanged(
    scratch: &Scratch,
    fixture: &Fixture,
    known: KnownSuppliesFrontiers,
    expected: SuppliesStoreError,
) {
    let before = std::fs::read(scratch.db()).unwrap();
    let names = || {
        let mut names: Vec<_> = std::fs::read_dir(scratch.db().parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    let names_before = names();
    let result = SuppliesStore::open_existing(scratch.db(), &fixture.policy, known);
    assert!(
        matches!(result, Err(actual) if actual == expected),
        "refuse this history before returning an owner"
    );
    assert_eq!(
        std::fs::read(scratch.db()).unwrap(),
        before,
        "refusal preserves the rejected file bytes"
    );
    assert_eq!(
        names(),
        names_before,
        "refusal leaves no replacement or sidecar at return"
    );
}
#[test]
fn replay_refuses_changed_operation_digest_and_nonconsecutive_revision() {
    for change_revision in [false, true] {
        let (scratch, fixture, known) = retained();
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        let issue = fixture.issuance();
        let changed = if change_revision {
            5_u64.to_be_bytes().to_vec()
        } else {
            let mut digest: Vec<u8> = sql
                .query_row(
                    "SELECT economic_digest FROM supplies_operations WHERE operation=?1",
                    [issue.issuance.as_bytes()],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(digest.len(), 32);
            digest[0] ^= 1;
            digest
        };
        let update = if change_revision {
            "UPDATE supplies_operations SET revision=?1 WHERE operation=?2"
        } else {
            "UPDATE supplies_operations SET economic_digest=?1 WHERE operation=?2"
        };
        assert_eq!(
            sql.execute(
                update,
                rusqlite::params![changed, issue.issuance.as_bytes()]
            )
            .unwrap(),
            1
        );
        drop(sql);
        refused_unchanged(&scratch, &fixture, known, SuppliesStoreError::Corrupt);
    }
}
#[test]
fn replay_refuses_changed_request_binding_and_operation_without_original_request() {
    for delete_original in [false, true] {
        let (scratch, fixture, known) = retained();
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        let issue = fixture.issuance();
        if delete_original {
            assert_eq!(
                sql.execute(
                    "DELETE FROM supplies_requests WHERE request=?1",
                    [issue.request.as_bytes()]
                )
                .unwrap(),
                1
            );
        } else {
            let mut binding: Vec<u8> = sql
                .query_row(
                    "SELECT binding_digest FROM supplies_requests WHERE request=?1",
                    [issue.request.as_bytes()],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(binding.len(), 32);
            binding[0] ^= 1;
            assert_eq!(
                sql.execute(
                    "UPDATE supplies_requests SET binding_digest=?1 WHERE request=?2",
                    rusqlite::params![binding, issue.request.as_bytes()]
                )
                .unwrap(),
                1
            );
        }
        drop(sql);
        refused_unchanged(&scratch, &fixture, known, SuppliesStoreError::Corrupt);
    }
}
#[test]
fn replay_refuses_cached_balance_that_disagrees_with_retained_events() {
    let (scratch, fixture, known) = retained();
    let sql = rusqlite::Connection::open(scratch.db()).unwrap();
    let mut body: Vec<u8> = sql
        .query_row("SELECT body FROM supplies_balances", [], |row| row.get(0))
        .unwrap();
    assert_eq!(body.len(), 48);
    body[0] ^= 1;
    assert_eq!(
        sql.execute("UPDATE supplies_balances SET body=?1", [body])
            .unwrap(),
        1
    );
    drop(sql);
    refused_unchanged(&scratch, &fixture, known, SuppliesStoreError::Corrupt);
}
#[test]
fn reopen_refuses_valid_older_database_below_observed_revision() {
    let scratch = Scratch::new();
    let fixture = Fixture::new();
    let store = SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
    let empty_known = store.known_frontiers().unwrap();
    assert_eq!(empty_known.revision, 0);
    drop(store);
    let valid_older = std::fs::read(scratch.db()).unwrap();
    let mut store =
        SuppliesStore::open_existing(scratch.db(), &fixture.policy, empty_known).unwrap();
    let issue = fixture.issuance();
    let proof = fixture.attempt(&mut store, ChallengeRequest::Issue(&issue));
    assert_eq!(
        store
            .issue(&issue, proof)
            .unwrap()
            .expect("signed later grant")
            .revision,
        1
    );
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 1);
    drop(store);
    std::fs::write(scratch.db(), valid_older).unwrap();
    refused_unchanged(&scratch, &fixture, known, SuppliesStoreError::StaleBackup);
}
