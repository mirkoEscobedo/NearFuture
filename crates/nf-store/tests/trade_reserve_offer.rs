#[path = "trade_reserve_offer/mod.rs"]
mod controls;
#[path = "trade_reserve_offer/reservation.rs"]
mod reservation;
#[path = "trade_support/reservation.rs"]
mod reservation_support;
#[path = "trade_reserve_offer/reserved_status.rs"]
mod reserved_status;
mod supplies_support;
mod trade_support;
use nf_contract::identity::RequestId;
use nf_kernel::trade::{TradeRejection, TradeRequestOutcome};
use nf_store::trade::{TradeChallenge, TradeStore, TradeStoreError as Error};
use supplies_support::Scratch;
use trade_support::TradeFixture;

#[test]
fn jointly_signed_offer_refusal_never_partially_locks_and_survives_later_funding() {
    let scratch = Scratch::new();
    let fixture = TradeFixture::new();
    let mut store =
        TradeStore::create(scratch.db(), &fixture.policy, &fixture.supplies.membership).unwrap();
    fixture.grant(&mut store, fixture.maker(), 25, 80, 81, 1);
    fixture.grant(&mut store, fixture.taker(), 4, 82, 83, 2);
    let before = store.known_frontiers().unwrap();
    assert_eq!(before.revision, 2);
    assert_eq!(before.clock_tick, 0);
    assert_eq!(before.clock_revision, 0);
    assert_eq!(
        fixture.balance(&mut store, fixture.maker()),
        [25, 0, 0, 0, 25, 0]
    );
    assert_eq!(
        fixture.balance(&mut store, fixture.taker()),
        [4, 0, 0, 0, 4, 0]
    );
    let offer = fixture.reserve_offer();
    let maker = fixture.attempt(&mut store, TradeChallenge::ReserveMaker(&offer));
    let taker = fixture.attempt(&mut store, TradeChallenge::ReserveTaker(&offer));
    assert_eq!(
        store.reserve_offer(&offer, maker, taker),
        Err(Error::Rejected(TradeRejection::InsufficientAvailable))
    );
    assert_eq!(
        fixture.balance(&mut store, fixture.maker()),
        [25, 0, 0, 0, 25, 0]
    );
    assert_eq!(
        fixture.balance(&mut store, fixture.taker()),
        [4, 0, 0, 0, 4, 0]
    );
    fixture.assert_no_open_offer_or_receipt(&mut store, &offer);
    let refused = store.known_frontiers().unwrap();
    assert_eq!(
        refused.revision, 3,
        "one admitted refusal, no partial escrow"
    );
    assert_eq!((refused.clock_tick, refused.clock_revision), (0, 0));
    let original = TradeRequestOutcome::Rejected {
        operation: offer.operation,
        revision: 3,
        reason: TradeRejection::InsufficientAvailable,
    };
    for party in [fixture.maker(), fixture.taker()] {
        assert_eq!(
            fixture.request_status(&mut store, party, offer.request),
            Some(original)
        );
    }
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &fixture.policy, refused).unwrap();
    fixture.grant(&mut store, fixture.taker(), 1, 86, 87, 4);
    let funded = store.known_frontiers().unwrap();
    assert_eq!(funded.revision, 4);
    let mut alias = offer;
    alias.request = RequestId::from_bytes([88; 16]);
    for value in [offer, alias] {
        let maker = fixture.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
        let taker = fixture.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
        assert_eq!(
            store.reserve_offer(&value, maker, taker),
            Err(Error::Rejected(TradeRejection::InsufficientAvailable))
        );
        assert_eq!(store.known_frontiers().unwrap(), funded);
        for party in [fixture.maker(), fixture.taker()] {
            assert_eq!(
                fixture.request_status(&mut store, party, value.request),
                Some(original)
            );
        }
    }
    assert_eq!(
        fixture.balance(&mut store, fixture.maker()),
        [25, 0, 0, 0, 25, 0]
    );
    assert_eq!(
        fixture.balance(&mut store, fixture.taker()),
        [5, 0, 0, 0, 5, 0]
    );
    fixture.assert_no_open_offer_or_receipt(&mut store, &offer);
    drop(store);
    let mut store = TradeStore::open_existing(scratch.db(), &fixture.policy, funded).unwrap();
    for request in [offer.request, alias.request] {
        for party in [fixture.maker(), fixture.taker()] {
            assert_eq!(
                fixture.request_status(&mut store, party, request),
                Some(original)
            );
        }
    }
    assert_eq!(
        fixture.balance(&mut store, fixture.maker()),
        [25, 0, 0, 0, 25, 0]
    );
    assert_eq!(
        fixture.balance(&mut store, fixture.taker()),
        [5, 0, 0, 0, 5, 0]
    );
    assert_eq!(store.known_frontiers().unwrap(), funded);
}
