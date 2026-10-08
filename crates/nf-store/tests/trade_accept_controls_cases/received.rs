use super::*;

#[test]
fn received_only_keys_support_joint_reserve_cancel_and_later_authority_grants() {
    let mut s = Scenario::new(false);
    let accepted = s.accepted();
    let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&s.accept));
    assert_eq!(s.store.accept_offer(&s.accept, proof), Ok(accepted));
    assert_eq!(
        s.f.balance(&mut s.store, s.f.maker(), s.f.w),
        snapshot([5, 0, 0, 0, 0, 0], 5, 0)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.taker(), s.f.g),
        snapshot([8, 0, 0, 0, 0, 0], 8, 0)
    );
    let mut reverse = s.reserve;
    reverse.request = RequestId::from_bytes([102; 16]);
    reverse.operation = OperationId::from_bytes([101; 16]);
    reverse.terms.offer = OfferId::from_bytes([100; 16]);
    reverse.terms.give.origin = s.f.w;
    reverse.terms.give.amount = 2;
    reverse.terms.want.origin = s.f.g;
    reverse.terms.want.amount = 3;
    let maker =
        s.f.attempt(&mut s.store, TradeChallenge::ReserveMaker(&reverse));
    let taker =
        s.f.attempt(&mut s.store, TradeChallenge::ReserveTaker(&reverse));
    let reserved = s.store.reserve_offer(&reverse, maker, taker).unwrap();
    assert_eq!(
        (reserved.offer, reserved.version, reserved.revision),
        (OfferId::from_bytes([100; 16]), 1, 5)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.maker(), s.f.w),
        snapshot([3, 2, 0, 0, 0, 0], 5, 0)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.taker(), s.f.g),
        snapshot([5, 3, 0, 0, 0, 0], 8, 0)
    );
    let cancel = s.cancel(reserved, s.f.taker(), (110, 111));
    let cancelled = TradeReceipt {
        kind: TradeFinalKind::Cancelled,
        operation: OperationId::from_bytes([111; 16]),
        revision: 6,
        offer: OfferId::from_bytes([100; 16]),
        version: 1,
        digest: reserved.digest,
    };
    let proof = s.f.attempt(&mut s.store, TradeChallenge::Cancel(&cancel));
    assert_eq!(s.store.cancel_offer(&cancel, proof), Ok(cancelled));
    assert_eq!(
        s.f.balance(&mut s.store, s.f.maker(), s.f.w),
        snapshot([5, 0, 0, 0, 0, 0], 5, 0)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.taker(), s.f.g),
        snapshot([8, 0, 0, 0, 0, 0], 8, 0)
    );
    s.f.grant(&mut s.store, s.f.maker(), s.f.w, 3, (120, 121), 7);
    s.f.grant(&mut s.store, s.f.taker(), s.f.g, 2, (122, 123), 8);
    let known = s.store.known_frontiers().unwrap();
    assert_eq!(
        (
            known.revision,
            known.membership_revision,
            known.clock_tick,
            known.clock_revision
        ),
        (8, 1, 0, 0)
    );
    for reopened in [false, true] {
        if reopened {
            drop(s.store);
            s.store = TradeStore::open_accepting(s.scratch.db(), &s.f.policy, known).unwrap();
        }
        assert_eq!(
            s.f.balance(&mut s.store, s.f.maker(), s.f.w),
            snapshot([8, 0, 0, 0, 3, 0], 5, 0)
        );
        assert_eq!(
            s.f.balance(&mut s.store, s.f.taker(), s.f.g),
            snapshot([10, 0, 0, 0, 2, 0], 8, 0)
        );
        assert_eq!(
            s.f.balance(&mut s.store, s.f.maker(), s.f.g),
            snapshot([17, 0, 0, 0, 25, 0], 0, 8)
        );
        assert_eq!(
            s.f.balance(&mut s.store, s.f.taker(), s.f.w),
            snapshot([5, 0, 0, 0, 10, 0], 0, 5)
        );
        let proof = s.f.attempt(&mut s.store, TradeChallenge::Cancel(&cancel));
        assert_eq!(s.store.cancel_offer(&cancel, proof), Ok(cancelled));
        let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&s.accept));
        assert_eq!(s.store.accept_offer(&s.accept, proof), Ok(accepted));
        for actor in [s.f.maker(), s.f.taker()] {
            assert_eq!(
                s.status(actor, reverse.request),
                Some(TradeRequestOutcome::Reserved {
                    operation: OperationId::from_bytes([101; 16]),
                    offer: reserved
                })
            );
            assert_eq!(
                s.status(actor, cancel.request),
                Some(TradeRequestOutcome::Cancelled(cancelled))
            );
            assert_eq!(
                s.status(actor, s.accept.request),
                Some(TradeRequestOutcome::Accepted(accepted))
            );
        }
        s.closed_and_outbox(accepted, &[accepted, cancelled]);
        s.closed_and_outbox(cancelled, &[accepted, cancelled]);
        assert_eq!(s.store.known_frontiers().unwrap(), known);
    }
}
