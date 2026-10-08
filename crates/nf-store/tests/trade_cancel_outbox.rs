mod cancel_outbox_support;
use cancel_outbox_support::{Keys, Scratch};
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::trade::{TradeFinalKind, TradeRejection, TradeRequestOutcome};
use nf_store::trade::{TradeChallenge, TradeStore, TradeStoreError as Error};

#[test]
fn cancellation_outbox_uses_revision_order_exclusive_cursor_and_party_filter_before_limit() {
    let scratch = Scratch::new();
    let keys = Keys::new();
    let mut store = keys.create(scratch.db());
    keys.grant(&mut store, 0, 25, 10, 11, 1);
    keys.grant(&mut store, 1, 10, 12, 13, 2);
    let (reserve_a, a) = keys.reserve(&mut store, 1, (30, 20, 21), (8, 5), 3);
    let (reserve_b, b) = keys.reserve(&mut store, 1, (31, 22, 23), (8, 5), 4);
    assert_eq!(keys.balance(&mut store, 0), [9, 16, 0, 0, 25, 0]);
    assert_eq!(keys.balance(&mut store, 1), [0, 10, 0, 0, 10, 0]);
    // Deliberately opposite OperationId/revision order: HIGH225 at5 precedes LOW17 at6.
    let (cancel_a, receipt_a) = keys.cancel(&mut store, a, 50, 225, 5);
    let (cancel_b, receipt_b) = keys.cancel(&mut store, b, 60, 17, 6);
    assert_eq!(receipt_a.kind, TradeFinalKind::Cancelled);
    assert_eq!(receipt_b.kind, TradeFinalKind::Cancelled);
    assert_eq!(receipt_a.operation, OperationId::from_bytes([225; 16]));
    assert_eq!(receipt_b.operation, OperationId::from_bytes([17; 16]));
    let known = store.known_frontiers().unwrap();
    assert_eq!((known.revision, known.membership_revision), (6, 2));
    assert_eq!((known.clock_tick, known.clock_revision), (0, 0));
    for (value, alias_request, receipt) in [(cancel_a, 51, receipt_a), (cancel_b, 61, receipt_b)] {
        let proof = keys.attempt(&mut store, TradeChallenge::Cancel(&value));
        assert_eq!(store.cancel_offer(&value, proof), Ok(receipt));
        let mut alias = value;
        alias.request = RequestId::from_bytes([alias_request; 16]);
        let proof = keys.attempt(&mut store, TradeChallenge::Cancel(&alias));
        assert_eq!(store.cancel_offer(&alias, proof), Ok(receipt));
        assert_eq!(store.known_frontiers().unwrap(), known);
    }
    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = TradeStore::open_existing(scratch.db(), &keys.policy, known).unwrap();
        }
        assert_eq!(keys.balance(&mut store, 0), [25, 0, 0, 0, 25, 0]);
        assert_eq!(keys.balance(&mut store, 1), [10, 0, 0, 0, 10, 0]);
        for actor in [0, 1] {
            for (value, reserved) in [(reserve_a, a), (reserve_b, b)] {
                assert_eq!(
                    keys.status(&mut store, actor, value.request),
                    Some(TradeRequestOutcome::Reserved {
                        operation: value.operation,
                        offer: reserved
                    })
                );
            }
            for (original, alias, receipt) in [(50, 51, receipt_a), (60, 61, receipt_b)] {
                for request in [original, alias] {
                    assert_eq!(
                        keys.status(&mut store, actor, RequestId::from_bytes([request; 16])),
                        Some(TradeRequestOutcome::Cancelled(receipt))
                    );
                }
            }
            for cursor in [0, 4, 5, 6, u64::MAX] {
                for limit in [1, 2, 64] {
                    let expected: Vec<_> = [receipt_a, receipt_b]
                        .into_iter()
                        .filter(|receipt| receipt.revision > cursor)
                        .take(limit as usize)
                        .collect();
                    assert_eq!(
                        keys.outbox(&mut store, actor, cursor, limit).unwrap(),
                        expected
                    );
                }
            }
            for limit in [0, 65] {
                assert_eq!(
                    keys.outbox(&mut store, actor, 0, limit),
                    Err(Error::Rejected(TradeRejection::Limit))
                );
            }
        }
        assert_eq!(keys.outbox(&mut store, 2, 0, 1).unwrap(), Vec::new());
        assert_eq!(store.known_frontiers().unwrap(), known);
    }
    // C was genuinely admitted to THIS repository, but neither initial receipt names C.
    // Its later real receipt must survive filtering before the limit1 truncation.
    keys.grant(&mut store, 2, 1, 70, 71, 7);
    let (_, c) = keys.reserve(&mut store, 2, (32, 72, 73), (1, 1), 8);
    let (_, receipt_c) = keys.cancel(&mut store, c, 74, 75, 9);
    assert_eq!(keys.balance(&mut store, 0), [25, 0, 0, 0, 25, 0]);
    assert_eq!(keys.balance(&mut store, 1), [10, 0, 0, 0, 10, 0]);
    assert_eq!(keys.balance(&mut store, 2), [1, 0, 0, 0, 1, 0]);
    let final_known = store.known_frontiers().unwrap();
    assert_eq!(
        (final_known.revision, final_known.membership_revision),
        (9, 2)
    );
    assert_eq!((final_known.clock_tick, final_known.clock_revision), (0, 0));
    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = TradeStore::open_existing(scratch.db(), &keys.policy, final_known).unwrap();
        }
        assert_eq!(keys.outbox(&mut store, 2, 0, 1).unwrap(), vec![receipt_c]);
        assert_eq!(keys.outbox(&mut store, 2, 0, 64).unwrap(), vec![receipt_c]);
        assert_eq!(keys.outbox(&mut store, 2, 9, 1).unwrap(), Vec::new());
        assert_eq!(
            keys.outbox(&mut store, 0, 0, 64).unwrap(),
            vec![receipt_a, receipt_b, receipt_c]
        );
        assert_eq!(
            keys.outbox(&mut store, 1, 0, 64).unwrap(),
            vec![receipt_a, receipt_b]
        );
        assert_eq!(store.known_frontiers().unwrap(), final_known);
    }
}
