mod supplies_support;
mod trade_accept_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::{
    supplies::{Balances, StatusQuery},
    trade::{
        AcceptOffer, OfferState, OfferStatusQuery, TradeBalance, TradeFinalKind, TradeOutboxQuery,
        TradeReceipt, TradeRequestOutcome, accept_offer_bytes, accept_offer_digest,
        acceptance_policy_bytes, decode_accept_offer, economic_accept_offer_digest,
        trade_policy_bytes,
    },
};
use nf_store::trade::{TradeChallenge, TradeStore};
use supplies_support::Scratch;
use trade_accept_support::AcceptFixture;

fn snapshot(stock: [u64; 6], received: u64, sent: u64) -> TradeBalance {
    TradeBalance {
        stock: Balances {
            available: stock[0],
            reserved: stock[1],
            pending: stock[2],
            externalized: stock[3],
            minted: stock[4],
            burned: stock[5],
        },
        received,
        sent,
    }
}
#[test]
fn named_taker_acceptance_exchanges_original_escrow_between_four_keys_once_after_reopen_and_alias()
{
    let scratch = Scratch::new();
    let f = AcceptFixture::new();
    let legacy = trade_policy_bytes(&f.policy.trade).unwrap();
    let policy3 = acceptance_policy_bytes(&f.policy).unwrap();
    let prefix = b"NF-TRADE-POLICY-3\0";
    assert!(policy3.starts_with(prefix));
    assert_eq!(
        &policy3[prefix.len()..prefix.len() + 4],
        &(legacy.len() as u32).to_le_bytes()
    );
    assert_eq!(
        &policy3[prefix.len() + 4..policy3.len() - 4],
        legacy.as_slice()
    );
    assert_eq!(&policy3[policy3.len() - 4..], &1_u32.to_le_bytes());
    assert_ne!(f.policy.digest().unwrap(), f.policy.trade.digest().unwrap());
    let mut store =
        TradeStore::create_accepting(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), f.g, 25, (80, 81), 1);
    f.grant(&mut store, f.taker(), f.w, 10, (82, 83), 2);
    let reserve = f.reserve();
    assert_eq!(reserve.terms.policy, f.policy.digest().unwrap());
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
    let reserved = store.reserve_offer(&reserve, maker, taker).unwrap();
    assert_eq!(reserved.revision, 3);
    assert_eq!(
        f.balance(&mut store, f.maker(), f.g),
        snapshot([17, 8, 0, 0, 25, 0], 0, 0)
    );
    assert_eq!(
        f.balance(&mut store, f.taker(), f.g),
        TradeBalance::default()
    );
    assert_eq!(
        f.balance(&mut store, f.taker(), f.w),
        snapshot([5, 5, 0, 0, 10, 0], 0, 0)
    );
    assert_eq!(
        f.balance(&mut store, f.maker(), f.w),
        TradeBalance::default()
    );
    let before = store.known_frontiers().unwrap();
    assert_eq!(before.revision, 3);
    assert_eq!((before.clock_tick, before.clock_revision), (0, 0));
    let value = AcceptOffer {
        request: RequestId::from_bytes([92; 16]),
        operation: OperationId::from_bytes([91; 16]),
        actor: f.taker(),
        device: f.supplies.issuer.device,
        universe: reserve.terms.universe,
        history: reserve.terms.history,
        policy: f.policy.digest().unwrap(),
        offer: reserved.offer,
        version: reserved.version,
        digest: reserved.digest,
    };
    let bytes = accept_offer_bytes(&value);
    assert_eq!(bytes.len(), 204);
    assert_eq!(decode_accept_offer(&bytes), Ok(value));
    let mut alias = value;
    alias.request = RequestId::from_bytes([88; 16]);
    assert_ne!(accept_offer_digest(&value), accept_offer_digest(&alias));
    assert_eq!(
        economic_accept_offer_digest(&value),
        economic_accept_offer_digest(&alias)
    );
    let expected = TradeReceipt {
        kind: TradeFinalKind::Accepted,
        operation: OperationId::from_bytes([91; 16]),
        revision: 4,
        offer: reserved.offer,
        version: 1,
        digest: reserved.digest,
    };
    let proof = f.attempt(&mut store, TradeChallenge::Accept(&value));
    // Protected first RED: real signed funded/open setup then literal Accepted4.
    assert_eq!(store.accept_offer(&value, proof), Ok(expected));
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 4);
    assert_eq!(known.membership_revision, before.membership_revision);
    assert_eq!((known.clock_tick, known.clock_revision), (0, 0));
    for request in [value, alias] {
        let proof = f.attempt(&mut store, TradeChallenge::Accept(&request));
        assert_eq!(store.accept_offer(&request, proof), Ok(expected));
        assert_eq!(store.known_frontiers().unwrap(), known);
    }
    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = TradeStore::open_accepting(scratch.db(), &f.policy, known).unwrap();
            // Exact and aliased acceptance retries use fresh one-use proofs AFTER durable reopen.
            for request in [value, alias] {
                let proof = f.attempt(&mut store, TradeChallenge::Accept(&request));
                assert_eq!(store.accept_offer(&request, proof), Ok(expected));
                assert_eq!(store.known_frontiers().unwrap(), known);
            }
        }
        let maker_g = f.balance(&mut store, f.maker(), f.g);
        let taker_g = f.balance(&mut store, f.taker(), f.g);
        let taker_w = f.balance(&mut store, f.taker(), f.w);
        let maker_w = f.balance(&mut store, f.maker(), f.w);
        assert_eq!(maker_g, snapshot([17, 0, 0, 0, 25, 0], 0, 8));
        assert_eq!(taker_g, snapshot([8, 0, 0, 0, 0, 0], 8, 0));
        assert_eq!(taker_w, snapshot([5, 0, 0, 0, 10, 0], 0, 5));
        assert_eq!(maker_w, snapshot([5, 0, 0, 0, 0, 0], 5, 0));
        assert_eq!(maker_g.stock.available + taker_g.stock.available, 25);
        assert_eq!(taker_w.stock.available + maker_w.stock.available, 10);
        assert_eq!((maker_g.sent, taker_g.received), (8, 8));
        assert_eq!((taker_w.sent, maker_w.received), (5, 5));
        for (actor, device) in [
            (f.maker(), f.supplies.beneficiary.device),
            (f.taker(), f.supplies.issuer.device),
        ] {
            let query = OfferStatusQuery {
                actor,
                device,
                universe: value.universe,
                history: value.history,
                offer: value.offer,
                version: 1,
            };
            let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&query));
            assert_eq!(
                store.offer_state(&query, proof).unwrap(),
                Some(OfferState::Closed(expected))
            );
            let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&query));
            assert_eq!(store.offer_status(&query, proof).unwrap(), None);
            for request in [reserve.request, value.request, alias.request] {
                let query = StatusQuery {
                    actor,
                    device,
                    owner: actor,
                    universe: value.universe,
                    history: value.history,
                    request,
                };
                let proof = f.attempt(&mut store, TradeChallenge::Status(&query));
                let outcome = store.status(&query, proof).unwrap();
                assert_eq!(
                    outcome,
                    Some(if request == reserve.request {
                        TradeRequestOutcome::Reserved {
                            operation: reserve.operation,
                            offer: reserved,
                        }
                    } else {
                        TradeRequestOutcome::Accepted(expected)
                    })
                );
            }
            let query = TradeOutboxQuery {
                actor,
                device,
                universe: value.universe,
                history: value.history,
                after_revision: 0,
                limit: 16,
            };
            let proof = f.attempt(&mut store, TradeChallenge::Outbox(&query));
            assert_eq!(store.outbox(&query, proof).unwrap(), vec![expected]);
        }
        let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
        let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
        assert_eq!(store.reserve_offer(&reserve, maker, taker), Ok(reserved));
        assert_eq!(store.known_frontiers().unwrap(), known);
    }
}
