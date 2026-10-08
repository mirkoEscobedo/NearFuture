use crate::{Error, Scratch, TradeChallenge, TradeFixture, TradeRejection, TradeStore};
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::{
    supplies::SuppliesRejection,
    trade::{OfferStatusQuery, ReserveOffer, ReservedOffer, TradeOutboxQuery, offer_terms_digest},
};
use nf_store::supplies::SuppliesStoreError;
fn reserved_snapshot(
    f: &TradeFixture,
    store: &mut TradeStore,
    value: &ReserveOffer,
    expected: ReservedOffer,
) {
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 3);
    assert_eq!((known.clock_tick, known.clock_revision), (0, 0));
    assert_eq!(f.balance(store, f.maker()), [17, 8, 0, 0, 25, 0]);
    assert_eq!(f.balance(store, f.taker()), [5, 5, 0, 0, 10, 0]);
    for (actor, device) in [
        (f.maker(), f.supplies.beneficiary.device),
        (f.taker(), f.supplies.issuer.device),
    ] {
        let query = OfferStatusQuery {
            actor,
            device,
            universe: value.terms.universe,
            history: value.terms.history,
            offer: value.terms.offer,
            version: value.terms.version,
        };
        let proof = f.attempt(store, TradeChallenge::OfferStatus(&query));
        assert_eq!(store.offer_status(&query, proof).unwrap(), Some(expected));
        let query = TradeOutboxQuery {
            actor,
            device,
            universe: value.terms.universe,
            history: value.terms.history,
            after_revision: 0,
            limit: 16,
        };
        let proof = f.attempt(store, TradeChallenge::Outbox(&query));
        assert!(
            store.outbox(&query, proof).unwrap().is_empty(),
            "reservation has no settlement receipt"
        );
    }
}
#[test]
fn funded_joint_offer_locks_each_side_once_across_reopen_and_aliases() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = TradeStore::create(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), 25, 80, 81, 1);
    f.grant(&mut store, f.taker(), 10, 82, 83, 2);
    let before = store.known_frontiers().unwrap();
    assert_eq!(before.revision, 2);
    assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [10, 0, 0, 0, 10, 0]);
    let value = f.reserve_offer();
    let expected = ReservedOffer {
        offer: value.terms.offer,
        version: 1,
        revision: 3,
        digest: offer_terms_digest(&value.terms),
    };
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
    // First behavioral RED: genuine funded public operation is currently UnsupportedOperation.
    assert_eq!(store.reserve_offer(&value, maker, taker), Ok(expected));
    reserved_snapshot(&f, &mut store, &value, expected);
    let committed = store.known_frontiers().unwrap();
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, committed).unwrap();
    let mut alias = value;
    alias.request = RequestId::from_bytes([88; 16]);
    for request in [value, alias] {
        let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&request));
        let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&request));
        assert_eq!(store.reserve_offer(&request, maker, taker), Ok(expected));
        assert_eq!(store.known_frontiers().unwrap(), committed);
    }
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, committed).unwrap();
    reserved_snapshot(&f, &mut store, &value, expected);
    // The alias must retain the shared durable request namespace across the second reopen.
    let mut collision = f.supplies.issuance();
    collision.request = alias.request;
    collision.issuance = OperationId::from_bytes([99; 16]);
    let proof = f.attempt(&mut store, TradeChallenge::Issue(&collision));
    assert_eq!(
        store.issue(&collision, proof),
        Err(Error::Supplies(SuppliesStoreError::Rejected(
            SuppliesRejection::Conflict
        )))
    );
    // Fresh operation IDs cannot relock this logical offer or replace its immutable version/terms.
    for version in [1, 2] {
        let mut duplicate = value;
        duplicate.request = RequestId::from_bytes([100 + version as u8; 16]);
        duplicate.operation = OperationId::from_bytes([110 + version as u8; 16]);
        duplicate.terms.version = version;
        let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&duplicate));
        let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&duplicate));
        assert_eq!(
            store.reserve_offer(&duplicate, maker, taker),
            Err(Error::Rejected(TradeRejection::Conflict))
        );
        for party in [f.maker(), f.taker()] {
            assert_eq!(f.request_status(&mut store, party, duplicate.request), None);
        }
    }
    let mut changed = alias;
    changed.request = RequestId::from_bytes([120; 16]);
    changed.terms.give.amount = 9;
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&changed));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&changed));
    assert_eq!(
        store.reserve_offer(&changed, maker, taker),
        Err(Error::Rejected(TradeRejection::Conflict))
    );
    assert_eq!(store.known_frontiers().unwrap(), committed);
    reserved_snapshot(&f, &mut store, &value, expected);
}
