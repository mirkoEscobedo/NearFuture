mod supplies_support;
mod trade_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::trade::{
    CancelOffer, OfferId, TradeRequestOutcome, cancel_offer_bytes, cancel_offer_digest,
    economic_cancel_offer_digest,
};
use nf_store::{
    supplies::SuppliesStoreError,
    trade::{KnownTradeFrontiers, TradeChallenge, TradeStore, TradeStoreError as Error},
};
use rusqlite::{Connection, params};
use supplies_support::Scratch;
use trade_support::TradeFixture;

fn committed_pair() -> (Scratch, TradeFixture, KnownTradeFrontiers, [CancelOffer; 2]) {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = TradeStore::create(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), 25, 80, 81, 1);
    f.grant(&mut store, f.taker(), 10, 82, 83, 2);
    let first = f.reserve_offer();
    let mut second = first;
    second.request = RequestId::from_bytes([102; 16]);
    second.operation = OperationId::from_bytes([101; 16]);
    second.terms.offer = OfferId::from_bytes([100; 16]);
    let mut cancels = Vec::new();
    let mut receipts = Vec::new();
    for (value, revision, request, operation) in [(first, 3, 110, 225), (second, 4, 120, 17)] {
        let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
        let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
        let reserved = store.reserve_offer(&value, maker, taker).unwrap();
        assert_eq!(reserved.revision, revision);
        cancels.push(CancelOffer {
            request: RequestId::from_bytes([request; 16]),
            operation: OperationId::from_bytes([operation; 16]),
            actor: f.maker(),
            device: f.supplies.beneficiary.device,
            universe: value.terms.universe,
            history: value.terms.history,
            policy: f.policy.digest().unwrap(),
            offer: reserved.offer,
            version: reserved.version,
            digest: reserved.digest,
        });
    }
    assert_eq!(f.balance(&mut store, f.maker()), [9, 16, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [0, 10, 0, 0, 10, 0]);
    for (index, value) in cancels.iter().enumerate() {
        let proof = f.attempt(&mut store, TradeChallenge::Cancel(value));
        let receipt = store.cancel_offer(value, proof).unwrap();
        assert_eq!(receipt.revision, 5 + index as u64);
        let mut alias = *value;
        alias.request = RequestId::from_bytes([111 + index as u8 * 10; 16]);
        let proof = f.attempt(&mut store, TradeChallenge::Cancel(&alias));
        assert_eq!(store.cancel_offer(&alias, proof), Ok(receipt));
        for actor in [f.maker(), f.taker()] {
            for request in [value.request, alias.request] {
                assert_eq!(
                    f.request_status(&mut store, actor, request),
                    Some(TradeRequestOutcome::Cancelled(receipt))
                );
            }
        }
        receipts.push(receipt);
    }
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 6);
    assert_eq!((known.clock_tick, known.clock_revision), (0, 0));
    assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [10, 0, 0, 0, 10, 0]);
    drop(store);
    // Every negative begins with an actual healthy reopened two-final journal at retained6.
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, known).unwrap();
    assert_eq!(store.known_frontiers().unwrap(), known);
    for (value, receipt) in cancels.iter().zip(receipts) {
        assert_eq!(
            f.request_status(&mut store, f.maker(), value.request),
            Some(TradeRequestOutcome::Cancelled(receipt))
        );
    }
    drop(store);
    (scratch, f, known, cancels.try_into().unwrap())
}

/// Refresh the exact canonical operation AND every original/alias request binding.
/// This prevents an obsolete digest or request cache from being the refusal oracle.
fn rewrite_cancel(sql: &Connection, value: CancelOffer, decision: i64) {
    assert_eq!(sql.execute(
        "UPDATE supplies_operations SET economic_digest=?1,body=?2,decision=?3 WHERE operation=?4",
        params![economic_cancel_offer_digest(&value), cancel_offer_bytes(&value),
            decision, value.operation.as_bytes()]).unwrap(), 1);
    let requests: Vec<Vec<u8>> = sql
        .prepare("SELECT request FROM supplies_requests WHERE operation=?1 ORDER BY request")
        .unwrap()
        .query_map([value.operation.as_bytes()], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        requests.len(),
        2,
        "the original and genuine one-use request alias both exist"
    );
    let mut original_seen = false;
    for request in requests {
        let id: [u8; 16] = request.try_into().unwrap();
        original_seen |= &id == value.request.as_bytes();
        let mut alias = value;
        alias.request = RequestId::from_bytes(id);
        assert_eq!(sql.execute(
            "UPDATE supplies_requests SET binding_digest=?1,body=?2 WHERE request=?3 AND operation=?4",
            params![cancel_offer_digest(&alias), cancel_offer_bytes(&alias),
                alias.request.as_bytes(), value.operation.as_bytes()]).unwrap(), 1);
    }
    assert!(original_seen);
}
fn balance_bytes(values: [u64; 6]) -> Vec<u8> {
    values.into_iter().flat_map(u64::to_be_bytes).collect()
}
fn coherent_cache(sql: &Connection, f: &TradeFixture, reserved: bool) {
    for (account, values) in [
        (
            f.maker(),
            if reserved {
                [17, 8, 0, 0, 25, 0]
            } else {
                [25, 0, 0, 0, 25, 0]
            },
        ),
        (
            f.taker(),
            if reserved {
                [5, 5, 0, 0, 10, 0]
            } else {
                [10, 0, 0, 0, 10, 0]
            },
        ),
    ] {
        assert_eq!(
            sql.execute(
                "UPDATE supplies_balances SET body=?1 WHERE account=?2",
                params![balance_bytes(values), account.as_bytes()]
            )
            .unwrap(),
            1
        );
    }
}
fn refused_unchanged(scratch: &Scratch, f: &TradeFixture, known: KnownTradeFrontiers) {
    let snapshot = || {
        let mut files: Vec<_> = std::fs::read_dir(scratch.db().parent().unwrap())
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), std::fs::read(entry.path()).unwrap())
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    };
    let before = snapshot();
    assert!(before.iter().any(|(name, _)| name == "supplies.sqlite"));
    let result = TradeStore::open_existing(scratch.db(), &f.policy, known);
    assert!(
        matches!(result, Err(Error::Supplies(SuppliesStoreError::Corrupt))),
        "semantic cancellation corruption must refuse exactly Corrupt"
    );
    assert_eq!(
        snapshot(),
        before,
        "database and every sidecar byte/name remain unchanged"
    );
}

#[test]
fn replay_refuses_a_cancel_forged_as_denied_despite_coherent_remaining_escrow() {
    let (scratch, f, known, values) = committed_pair();
    let sql = Connection::open(scratch.db()).unwrap();
    let tx = sql.unchecked_transaction().unwrap();
    rewrite_cancel(&tx, values[0], 1);
    coherent_cache(&tx, &f, true);
    tx.commit().unwrap();
    drop(sql);
    refused_unchanged(&scratch, &f, known);
}
#[test]
fn replay_refuses_a_second_final_retargeted_to_a_closed_offer_despite_other_equal_escrow() {
    let (scratch, f, known, values) = committed_pair();
    let sql = Connection::open(scratch.db()).unwrap();
    let tx = sql.unchecked_transaction().unwrap();
    let mut second = values[1];
    second.offer = values[0].offer;
    second.version = values[0].version;
    second.digest = values[0].digest;
    rewrite_cancel(&tx, second, 0);
    coherent_cache(&tx, &f, false);
    tx.commit().unwrap();
    drop(sql);
    refused_unchanged(&scratch, &f, known);
}
#[test]
fn replay_refuses_a_changed_original_target_digest_with_all_aliases_rebound() {
    let (scratch, f, known, values) = committed_pair();
    let sql = Connection::open(scratch.db()).unwrap();
    let tx = sql.unchecked_transaction().unwrap();
    let mut first = values[0];
    first.digest[0] ^= 1;
    rewrite_cancel(&tx, first, 0);
    coherent_cache(&tx, &f, false);
    tx.commit().unwrap();
    drop(sql);
    refused_unchanged(&scratch, &f, known);
}
