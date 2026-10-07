mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{
    Burn, BurnOutcome, BurnReason, RejectedOutcome, RequestOutcome, RequestRejection, Reserve,
    ReserveOutcome, StatusQuery, SuppliesRejection,
};
use nf_store::supplies::{
    ChallengeRequest, KnownSuppliesFrontiers, SuppliesStore, SuppliesStoreError as Error,
};
use supplies_support::{Fixture, Scratch};
fn retained(
    rejected: bool,
    burn: bool,
) -> (
    Scratch,
    Fixture,
    KnownSuppliesFrontiers,
    OperationId,
    RequestId,
) {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let issue = f.issuance();
    let q = f.query();
    if !rejected {
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issue));
        assert_eq!(store.issue(&issue, proof).unwrap().unwrap().revision, 1);
    }
    let request = RequestId::from_bytes([80; 16]);
    let operation = OperationId::from_bytes([81; 16]);
    let actual = if burn {
        let v = Burn {
            request,
            burn: operation,
            actor: issue.actor,
            device: issue.device,
            owner: q.owner,
            universe: q.universe,
            history: q.history,
            policy: issue.policy,
            content: q.content,
            origin: q.origin,
            reason: BurnReason::AuthorityDestruction,
            amount: 8,
        };
        let proof = f.attempt(&mut store, ChallengeRequest::Burn(&v));
        store.burn(&v, proof).map(|_| ())
    } else {
        let v = Reserve {
            request,
            reservation: operation,
            actor: q.actor,
            device: q.device,
            owner: q.owner,
            universe: q.universe,
            history: q.history,
            policy: issue.policy,
            content: q.content,
            origin: q.origin,
            amount: 8,
        };
        let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&v));
        store.reserve(&v, proof).map(|_| ())
    };
    let expected = if rejected {
        Err(Error::Rejected(SuppliesRejection::Limit))
    } else {
        Ok(())
    };
    assert_eq!(actual, expected);
    if rejected {
        let proof = f.attempt(&mut store, ChallengeRequest::Issue(&issue));
        assert_eq!(store.issue(&issue, proof).unwrap().unwrap().revision, 2);
    }
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 2);
    drop(store);
    let mut store = SuppliesStore::open_existing(scratch.db(), &f.policy, known).unwrap();
    let query = StatusQuery {
        actor: q.actor,
        device: q.device,
        owner: q.owner,
        universe: q.universe,
        history: q.history,
        request,
    };
    let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
    let outcome = if rejected {
        RequestOutcome::Rejected(RejectedOutcome {
            operation,
            revision: 1,
            reason: RequestRejection::InsufficientAvailable,
        })
    } else if burn {
        RequestOutcome::Burned(BurnOutcome {
            burn: operation,
            revision: 2,
        })
    } else {
        RequestOutcome::Reserved(ReserveOutcome {
            reservation: operation,
            revision: 2,
        })
    };
    assert_eq!(store.status(&query, proof).unwrap(), Some(outcome));
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&q));
    let b = store.balance(&q, proof).unwrap();
    let expected = if rejected {
        (25, 0, 0, 0, 25, 0)
    } else if burn {
        (17, 0, 0, 0, 25, 8)
    } else {
        (17, 8, 0, 0, 25, 0)
    };
    assert_eq!(
        (
            b.available,
            b.reserved,
            b.pending,
            b.externalized,
            b.minted,
            b.burned
        ),
        expected
    );
    drop(store);
    (scratch, f, known, operation, request)
}
fn refused_unchanged(scratch: &Scratch, f: &Fixture, known: KnownSuppliesFrontiers) {
    let before = std::fs::read(scratch.db()).unwrap();
    let names = || {
        let mut entries: Vec<_> = std::fs::read_dir(scratch.db().parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        entries.sort();
        entries
    };
    let entries = names();
    let result = SuppliesStore::open_existing(scratch.db(), &f.policy, known);
    assert!(
        matches!(result, Err(Error::Corrupt)),
        "tampered business decision must refuse before returning an owner"
    );
    assert_eq!(std::fs::read(scratch.db()).unwrap(), before);
    assert_eq!(names(), entries, "no replacement or sidecar on refusal");
}
#[test]
fn replay_refuses_an_underfunded_refusal_changed_into_an_asset_effect() {
    for burn in [false, true] {
        let (scratch, f, known, operation, _) = retained(true, burn);
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        assert_eq!(
            sql.execute(
                "UPDATE supplies_operations SET decision=0 WHERE operation=?1",
                [operation.as_bytes()]
            )
            .unwrap(),
            1
        );
        drop(sql);
        refused_unchanged(&scratch, &f, known);
    }
}
#[test]
fn replay_refuses_a_funded_asset_effect_forged_as_an_insufficient_refusal() {
    for burn in [false, true] {
        let (scratch, f, known, operation, _) = retained(false, burn);
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        assert_eq!(
            sql.execute(
                "UPDATE supplies_operations SET decision=1 WHERE operation=?1",
                [operation.as_bytes()]
            )
            .unwrap(),
            1
        );
        drop(sql);
        refused_unchanged(&scratch, &f, known);
    }
}
#[test]
fn replay_requires_the_original_request_of_an_immutable_refusal() {
    for burn in [false, true] {
        let (scratch, f, known, _, request) = retained(true, burn);
        let sql = rusqlite::Connection::open(scratch.db()).unwrap();
        assert_eq!(
            sql.execute(
                "DELETE FROM supplies_requests WHERE request=?1",
                [request.as_bytes()]
            )
            .unwrap(),
            1
        );
        drop(sql);
        refused_unchanged(&scratch, &f, known);
    }
}
