mod supplies_support;
mod trade_support;

use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::trade::{
    CancelOffer, OfferId, OfferState, OfferStatusQuery, TradeFinalKind, TradeOutboxQuery,
    TradeReceipt, TradeRejection, TradeRequestOutcome,
};
use nf_store::trade::{TradeChallenge, TradeStore, TradeStoreError as Error};
use supplies_support::Scratch;
use trade_support::TradeFixture;

#[test]
fn taker_cancellation_preserves_another_joint_escrow_after_reopen() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = TradeStore::create(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), 25, 80, 81, 1);
    f.grant(&mut store, f.taker(), 10, 82, 83, 2);
    let first = f.reserve_offer();
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&first));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&first));
    let first_reserved = store.reserve_offer(&first, maker, taker).unwrap();
    assert_eq!(first_reserved.offer, OfferId::from_bytes([90; 16]));
    assert_eq!(first_reserved.version, 1);
    assert_eq!(first_reserved.revision, 3);
    let mut second = first;
    second.request = RequestId::from_bytes([102; 16]);
    second.operation = OperationId::from_bytes([101; 16]);
    second.terms.offer = OfferId::from_bytes([100; 16]);
    second.terms.give.amount = 3;
    second.terms.want.amount = 2;
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&second));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&second));
    let second_reserved = store.reserve_offer(&second, maker, taker).unwrap();
    assert_eq!(second_reserved.offer, OfferId::from_bytes([100; 16]));
    assert_eq!(second_reserved.version, 1);
    assert_eq!(second_reserved.revision, 4);
    assert_eq!(f.balance(&mut store, f.maker()), [14, 11, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [3, 7, 0, 0, 10, 0]);
    let before = store.known_frontiers().unwrap();
    assert_eq!(before.revision, 4);

    let cancel = CancelOffer {
        request: RequestId::from_bytes([110; 16]),
        operation: OperationId::from_bytes([111; 16]),
        actor: f.taker(),
        device: f.supplies.issuer.device,
        universe: first.terms.universe,
        history: first.terms.history,
        policy: f.policy.digest().unwrap(),
        offer: first_reserved.offer,
        version: 1,
        digest: first_reserved.digest,
    };
    let receipt = TradeReceipt {
        kind: TradeFinalKind::Cancelled,
        operation: OperationId::from_bytes([111; 16]),
        revision: 5,
        offer: OfferId::from_bytes([90; 16]),
        version: 1,
        digest: first_reserved.digest,
    };
    let proof = f.attempt(&mut store, TradeChallenge::Cancel(&cancel));
    assert_eq!(store.cancel_offer(&cancel, proof), Ok(receipt));
    let committed = store.known_frontiers().unwrap();
    assert_eq!(committed.revision, 5);
    assert_eq!(committed.membership_revision, before.membership_revision);
    assert_eq!((committed.clock_tick, committed.clock_revision), (0, 0));

    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = TradeStore::open_existing(scratch.db(), &f.policy, committed).unwrap();
        }
        // Both offers used the same two balance keys; only the first8/5 escrow was released.
        assert_eq!(f.balance(&mut store, f.maker()), [22, 3, 0, 0, 25, 0]);
        assert_eq!(f.balance(&mut store, f.taker()), [8, 2, 0, 0, 10, 0]);
        for (actor, device) in [
            (f.maker(), f.supplies.beneficiary.device),
            (f.taker(), f.supplies.issuer.device),
        ] {
            for (offer, expected) in [
                (first.terms.offer, OfferState::Closed(receipt)),
                (second.terms.offer, OfferState::Reserved(second_reserved)),
            ] {
                let query = OfferStatusQuery {
                    actor,
                    device,
                    universe: first.terms.universe,
                    history: first.terms.history,
                    offer,
                    version: 1,
                };
                let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&query));
                assert_eq!(store.offer_state(&query, proof).unwrap(), Some(expected));
            }
            assert_eq!(
                f.request_status(&mut store, actor, second.request),
                Some(TradeRequestOutcome::Reserved {
                    operation: OperationId::from_bytes([101; 16]),
                    offer: second_reserved,
                })
            );
            let query = TradeOutboxQuery {
                actor,
                device,
                universe: first.terms.universe,
                history: first.terms.history,
                after_revision: 0,
                limit: 16,
            };
            let proof = f.attempt(&mut store, TradeChallenge::Outbox(&query));
            assert_eq!(store.outbox(&query, proof).unwrap(), vec![receipt]);
        }
        assert_eq!(store.known_frontiers().unwrap(), committed);
    }
}

#[test]
fn closed_offer_reserve_retries_preserve_history_and_fresh_versions_cannot_relock() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = TradeStore::create(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), 25, 80, 81, 1);
    f.grant(&mut store, f.taker(), 10, 82, 83, 2);
    let reserve = f.reserve_offer();
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
    let reserved = store.reserve_offer(&reserve, maker, taker).unwrap();
    assert_eq!(reserved.revision, 3);
    let cancel = CancelOffer {
        request: RequestId::from_bytes([92; 16]),
        operation: OperationId::from_bytes([91; 16]),
        actor: f.maker(),
        device: f.supplies.beneficiary.device,
        universe: reserve.terms.universe,
        history: reserve.terms.history,
        policy: f.policy.digest().unwrap(),
        offer: reserved.offer,
        version: 1,
        digest: reserved.digest,
    };
    let receipt = TradeReceipt {
        kind: TradeFinalKind::Cancelled,
        operation: OperationId::from_bytes([91; 16]),
        revision: 4,
        offer: OfferId::from_bytes([90; 16]),
        version: 1,
        digest: reserved.digest,
    };
    let proof = f.attempt(&mut store, TradeChallenge::Cancel(&cancel));
    assert_eq!(store.cancel_offer(&cancel, proof), Ok(receipt));
    let committed = store.known_frontiers().unwrap();
    assert_eq!(committed.revision, 4);
    assert_eq!((committed.clock_tick, committed.clock_revision), (0, 0));
    let mut alias = reserve;
    alias.request = RequestId::from_bytes([88; 16]);
    for value in [reserve, alias] {
        let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
        let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
        assert_eq!(store.reserve_offer(&value, maker, taker), Ok(reserved));
        assert_eq!(store.known_frontiers().unwrap(), committed);
    }
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, committed).unwrap();
    for actor in [f.maker(), f.taker()] {
        for request in [reserve.request, alias.request] {
            assert_eq!(
                f.request_status(&mut store, actor, request),
                Some(TradeRequestOutcome::Reserved {
                    operation: reserve.operation,
                    offer: reserved
                })
            );
        }
    }
    assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [10, 0, 0, 0, 10, 0]);

    for version in [1_u32, 2] {
        for insufficient in [false, true] {
            let code = version as u8 * 2 + u8::from(insufficient);
            let mut fresh = reserve;
            fresh.request = RequestId::from_bytes([120 + code; 16]);
            fresh.operation = OperationId::from_bytes([130 + code; 16]);
            fresh.terms.version = version;
            if insufficient {
                fresh.terms.give.amount = 26;
                fresh.terms.want.amount = 11;
            }
            let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&fresh));
            let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&fresh));
            assert_eq!(
                store.reserve_offer(&fresh, maker, taker),
                Err(Error::Rejected(TradeRejection::Conflict)),
                "a closed ID cannot become a stock refusal or a successor version"
            );
            for actor in [f.maker(), f.taker()] {
                assert_eq!(f.request_status(&mut store, actor, fresh.request), None);
            }
            assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
            assert_eq!(f.balance(&mut store, f.taker()), [10, 0, 0, 0, 10, 0]);
            assert_eq!(store.known_frontiers().unwrap(), committed);
        }
    }
}
