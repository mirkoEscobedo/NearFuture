use crate::{Scratch, TradeChallenge, TradeFixture, TradeRequestOutcome, TradeStore};
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::trade::{OfferId, ReservedOffer, offer_terms_digest};

#[test]
fn signed_reserved_status_retains_original_operation_for_both_parties_and_request_alias() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = TradeStore::create(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), 25, 80, 81, 1);
    f.grant(&mut store, f.taker(), 10, 82, 83, 2);
    let value = f.reserve_offer();
    let reserved = ReservedOffer {
        offer: OfferId::from_bytes([90; 16]),
        version: 1,
        revision: 3,
        digest: offer_terms_digest(&value.terms),
    };
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
    assert_eq!(store.reserve_offer(&value, maker, taker), Ok(reserved));
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 3);
    assert_eq!((known.clock_tick, known.clock_revision), (0, 0));
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, known).unwrap();
    let mut alias = value;
    alias.request = RequestId::from_bytes([88; 16]);
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&alias));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&alias));
    assert_eq!(store.reserve_offer(&alias, maker, taker), Ok(reserved));
    assert_eq!(store.known_frontiers().unwrap(), known);
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &f.policy, known).unwrap();
    let expected = TradeRequestOutcome::Reserved {
        operation: OperationId::from_bytes([85; 16]),
        offer: reserved,
    };
    for actor in [f.maker(), f.taker()] {
        for request in [
            RequestId::from_bytes([84; 16]),
            RequestId::from_bytes([88; 16]),
        ] {
            // Each lookup issues and signs a fresh real Status proof for this recorded party.
            assert_eq!(f.request_status(&mut store, actor, request), Some(expected));
        }
    }
    assert_eq!(f.balance(&mut store, f.maker()), [17, 8, 0, 0, 25, 0]);
    assert_eq!(f.balance(&mut store, f.taker()), [5, 5, 0, 0, 10, 0]);
    assert_eq!(store.known_frontiers().unwrap(), known);
}
