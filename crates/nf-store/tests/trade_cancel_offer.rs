mod supplies_support;
mod trade_support;

use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::trade::{
    CancelOffer, OfferId, OfferState, OfferStatusQuery, TradeFinalKind, TradeOutboxQuery,
    TradeReceipt, TradeRequestOutcome,
};
use nf_store::trade::{TradeChallenge, TradeStore};
use supplies_support::Scratch;
use trade_support::TradeFixture;

#[test]
fn authorized_named_party_cancellation_releases_joint_escrow_once_after_reopen() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = TradeStore::create(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    // funded() deliberately supplies only4 to the taker; this contract needs actual funded escrow.
    f.grant(&mut store, f.maker(), 25, 80, 81, 1);
    f.grant(&mut store, f.taker(), 10, 82, 83, 2);
    let reserve = f.reserve_offer();
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
    let reserved = store.reserve_offer(&reserve, maker, taker).unwrap();
    assert_eq!(reserved.offer, OfferId::from_bytes([90; 16]));
    assert_eq!(reserved.version, 1);
    assert_eq!(reserved.revision, 3);
    assert_eq!(f.balance(&mut store, f.maker()), [17, 8, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [5, 5, 0, 0, 10, 0]);

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
    // First behavioral RED must reach the authenticated UnsupportedOperation seam.
    let proof = f.attempt(&mut store, TradeChallenge::Cancel(&cancel));
    assert_eq!(store.cancel_offer(&cancel, proof), Ok(receipt));
    assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [10, 0, 0, 0, 10, 0]);
    let committed = store.known_frontiers().unwrap();
    assert_eq!(committed.revision, 4);
    assert_eq!((committed.clock_tick, committed.clock_revision), (0, 0));
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, committed).unwrap();
    let mut alias = cancel;
    alias.request = RequestId::from_bytes([93; 16]);
    for value in [cancel, alias] {
        let proof = f.attempt(&mut store, TradeChallenge::Cancel(&value));
        assert_eq!(store.cancel_offer(&value, proof), Ok(receipt));
        assert_eq!(store.known_frontiers().unwrap(), committed);
    }
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, committed).unwrap();
    for (actor, device) in [
        (f.maker(), f.supplies.beneficiary.device),
        (f.taker(), f.supplies.issuer.device),
    ] {
        assert_eq!(
            f.request_status(&mut store, actor, reserve.request),
            Some(TradeRequestOutcome::Reserved {
                operation: reserve.operation,
                offer: reserved
            }),
            "cancellation never rewrites the original reservation decision"
        );
        for request in [cancel.request, alias.request] {
            assert_eq!(
                f.request_status(&mut store, actor, request),
                Some(TradeRequestOutcome::Cancelled(receipt))
            );
        }
        let query = OfferStatusQuery {
            actor,
            device,
            universe: cancel.universe,
            history: cancel.history,
            offer: cancel.offer,
            version: 1,
        };
        // Byte-identical query projections each consume a separately issued signed proof.
        let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&query));
        assert_eq!(
            store.offer_state(&query, proof).unwrap(),
            Some(OfferState::Closed(receipt))
        );
        let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&query));
        assert_eq!(store.offer_status(&query, proof).unwrap(), None);
        let unknown = OfferStatusQuery {
            offer: OfferId::from_bytes([94; 16]),
            ..query
        };
        let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&unknown));
        assert_eq!(store.offer_state(&unknown, proof).unwrap(), None);
        for after_revision in [0, 3, 4] {
            let query = TradeOutboxQuery {
                actor,
                device,
                universe: cancel.universe,
                history: cancel.history,
                after_revision,
                limit: 1,
            };
            let proof = f.attempt(&mut store, TradeChallenge::Outbox(&query));
            let expected = if after_revision < 4 {
                vec![receipt]
            } else {
                Vec::new()
            };
            assert_eq!(store.outbox(&query, proof).unwrap(), expected);
        }
    }
    assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [10, 0, 0, 0, 10, 0]);
    assert_eq!(store.known_frontiers().unwrap(), committed);
}
