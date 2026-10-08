mod supplies_support;
mod trade_accept_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::{
    supplies::{
        Balances, Burn, BurnOutcome, BurnReason, RequestOutcome, SUPPLIES_CONTENT, StatusQuery,
        policy_digest,
    },
    trade::{
        AcceptOffer, OfferState, OfferStatusQuery, TradeBalance, TradeFinalKind, TradeOutboxQuery,
        TradeReceipt, TradeRequestOutcome,
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
fn authority_burns_received_only_stock_once_with_immutable_alias_status_after_reopen() {
    let scratch = Scratch::new();
    let f = AcceptFixture::new();
    // The existing genuine bootstrap authority is also the named taker; no new rule or role is invented.
    assert!(
        f.policy
            .trade
            .supplies
            .burners
            .iter()
            .any(|rule| rule.issuer == f.taker()
                && rule.origin == f.g
                && rule.content == SUPPLIES_CONTENT
                && rule.reason == BurnReason::AuthorityDestruction
                && rule.maximum >= 3)
    );
    let mut store =
        TradeStore::create_accepting(scratch.db(), &f.policy, &f.supplies.membership).unwrap();
    f.grant(&mut store, f.maker(), f.g, 25, (80, 81), 1);
    f.grant(&mut store, f.taker(), f.w, 10, (82, 83), 2);
    let reserve = f.reserve();
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&reserve));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&reserve));
    let reserved = store.reserve_offer(&reserve, maker, taker).unwrap();
    assert_eq!(reserved.revision, 3);
    let accept = AcceptOffer {
        request: RequestId::from_bytes([92; 16]),
        operation: OperationId::from_bytes([91; 16]),
        actor: f.taker(),
        device: f.supplies.issuer.device,
        universe: reserve.terms.universe,
        history: reserve.terms.history,
        policy: f.policy.digest().unwrap(),
        offer: reserved.offer,
        version: 1,
        digest: reserved.digest,
    };
    let accepted = TradeReceipt {
        kind: TradeFinalKind::Accepted,
        operation: OperationId::from_bytes([91; 16]),
        revision: 4,
        offer: reserved.offer,
        version: 1,
        digest: reserved.digest,
    };
    let proof = f.attempt(&mut store, TradeChallenge::Accept(&accept));
    assert_eq!(store.accept_offer(&accept, proof), Ok(accepted));
    assert_eq!(
        f.balance(&mut store, f.taker(), f.g),
        snapshot([8, 0, 0, 0, 0, 0], 8, 0)
    );
    assert_eq!(
        f.balance(&mut store, f.maker(), f.g),
        snapshot([17, 0, 0, 0, 25, 0], 0, 8)
    );
    assert_eq!(
        f.balance(&mut store, f.taker(), f.w),
        snapshot([5, 0, 0, 0, 10, 0], 0, 5)
    );
    assert_eq!(
        f.balance(&mut store, f.maker(), f.w),
        snapshot([5, 0, 0, 0, 0, 0], 5, 0)
    );
    let before = store.known_frontiers().unwrap();
    assert_eq!(
        (
            before.revision,
            before.membership_revision,
            before.clock_tick,
            before.clock_revision
        ),
        (4, 1, 0, 0)
    );
    let value = Burn {
        request: RequestId::from_bytes([164; 16]),
        burn: OperationId::from_bytes([165; 16]),
        actor: f.taker(),
        device: f.supplies.issuer.device,
        owner: f.taker(),
        universe: reserve.terms.universe,
        history: reserve.terms.history,
        policy: policy_digest(&f.policy.trade.supplies).unwrap(),
        content: SUPPLIES_CONTENT,
        origin: f.g,
        reason: BurnReason::AuthorityDestruction,
        amount: 3,
    };
    let issued = store.issue_challenge(TradeChallenge::Burn(&value)).unwrap();
    assert_eq!(issued.template.account, f.taker());
    assert_eq!(issued.template.device, f.supplies.issuer.device);
    assert_eq!(issued.template.frontier, 1);
    let proof = f.supplies.sign_issued(issued);
    let expected = BurnOutcome {
        burn: OperationId::from_bytes([165; 16]),
        revision: 5,
    };
    // Protected financial first RED follows real current authority proof and accepted received-only stock.
    assert_eq!(store.burn(&value, proof), Ok(Some(expected)));
    let known = store.known_frontiers().unwrap();
    assert_eq!(
        (
            known.revision,
            known.membership_revision,
            known.clock_tick,
            known.clock_revision
        ),
        (5, 1, 0, 0)
    );
    let mut alias = value;
    alias.request = RequestId::from_bytes([166; 16]);
    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = TradeStore::open_accepting(scratch.db(), &f.policy, known).unwrap();
        }
        for request in [value, alias] {
            let proof = f.attempt(&mut store, TradeChallenge::Burn(&request));
            assert_eq!(store.burn(&request, proof), Ok(Some(expected)));
            assert_eq!(store.known_frontiers().unwrap(), known);
        }
        let received = f.balance(&mut store, f.taker(), f.g);
        assert_eq!(received, snapshot([5, 0, 0, 0, 0, 3], 8, 0));
        let maker_g = f.balance(&mut store, f.maker(), f.g);
        let taker_w = f.balance(&mut store, f.taker(), f.w);
        let maker_w = f.balance(&mut store, f.maker(), f.w);
        assert_eq!(maker_g, snapshot([17, 0, 0, 0, 25, 0], 0, 8));
        assert_eq!(taker_w, snapshot([5, 0, 0, 0, 10, 0], 0, 5));
        assert_eq!(maker_w, snapshot([5, 0, 0, 0, 0, 0], 5, 0));
        assert_eq!(
            maker_g.stock.available + received.stock.available + received.stock.burned,
            25
        );
        assert_eq!(taker_w.stock.available + maker_w.stock.available, 10);
        assert_eq!((maker_g.sent, received.received), (8, 8));
        assert_eq!((taker_w.sent, maker_w.received), (5, 5));
        let query = f.query(f.taker(), f.g);
        let proof = f.attempt(&mut store, TradeChallenge::Balance(&query));
        assert_eq!(
            store.balance(&query, proof).unwrap(),
            snapshot([5, 0, 0, 0, 0, 3], 8, 0).stock
        );
        for (actor, device) in [
            (f.maker(), f.supplies.beneficiary.device),
            (f.taker(), f.supplies.issuer.device),
        ] {
            let mut requests = vec![
                (
                    reserve.request,
                    TradeRequestOutcome::Reserved {
                        operation: OperationId::from_bytes([85; 16]),
                        offer: reserved,
                    },
                ),
                (accept.request, TradeRequestOutcome::Accepted(accepted)),
            ];
            if actor == f.taker() {
                for request in [value.request, alias.request] {
                    requests.push((
                        request,
                        TradeRequestOutcome::Supplies(RequestOutcome::Burned(expected)),
                    ));
                }
            }
            for (request, outcome) in requests {
                let query = StatusQuery {
                    actor,
                    device,
                    owner: actor,
                    universe: value.universe,
                    history: value.history,
                    request,
                };
                let proof = f.attempt(&mut store, TradeChallenge::Status(&query));
                assert_eq!(store.status(&query, proof).unwrap(), Some(outcome));
            }
            let query = OfferStatusQuery {
                actor,
                device,
                universe: value.universe,
                history: value.history,
                offer: reserved.offer,
                version: 1,
            };
            let proof = f.attempt(&mut store, TradeChallenge::OfferStatus(&query));
            assert_eq!(
                store.offer_state(&query, proof).unwrap(),
                Some(OfferState::Closed(accepted))
            );
            let query = TradeOutboxQuery {
                actor,
                device,
                universe: value.universe,
                history: value.history,
                after_revision: 0,
                limit: 64,
            };
            let proof = f.attempt(&mut store, TradeChallenge::Outbox(&query));
            assert_eq!(store.outbox(&query, proof).unwrap(), vec![accepted]);
        }
        assert_eq!(store.known_frontiers().unwrap(), known);
    }
}
