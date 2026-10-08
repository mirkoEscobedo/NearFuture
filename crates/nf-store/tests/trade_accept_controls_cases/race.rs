use super::*;

#[test]
fn signed_accept_cancel_race_has_one_finality_on_one_canonical_owner() {
    let mut s = Scenario::new(false);
    let accept = s.accept;
    let cancel = s.cancel(s.reserved, s.f.maker(), (110, 111));
    let accepted = s.accepted();
    let cancelled = TradeReceipt {
        kind: TradeFinalKind::Cancelled,
        operation: OperationId::from_bytes([111; 16]),
        revision: 4,
        offer: s.reserved.offer,
        version: 1,
        digest: s.reserved.digest,
    };
    let accept_proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&accept));
    let cancel_proof = s.f.attempt(&mut s.store, TradeChallenge::Cancel(&cancel));
    let accept_replay = copy_proof(&accept_proof);
    let cancel_replay = copy_proof(&cancel_proof);
    // Two genuine producers compete for ONE canonical owner's serialized financial transaction.
    let owner = Mutex::new(&mut s.store);
    let barrier = Barrier::new(3);
    let (a, c) = std::thread::scope(|scope| {
        let canonical = &owner;
        let start = &barrier;
        let accepting = scope.spawn(move || {
            start.wait();
            canonical
                .lock()
                .unwrap()
                .accept_offer(&accept, accept_proof)
        });
        let cancelling = scope.spawn(move || {
            start.wait();
            canonical
                .lock()
                .unwrap()
                .cancel_offer(&cancel, cancel_proof)
        });
        barrier.wait();
        (accepting.join().unwrap(), cancelling.join().unwrap())
    });
    drop(owner);
    let conflict = Error::Rejected(TradeRejection::Conflict);
    let winner = match (a, c) {
        (Ok(receipt), Err(error)) => {
            assert_eq!(receipt, accepted);
            assert_eq!(error, conflict);
            accepted
        }
        (Err(error), Ok(receipt)) => {
            assert_eq!(error, conflict);
            assert_eq!(receipt, cancelled);
            cancelled
        }
        other => panic!("one successful final and one exact semantic conflict: {other:?}"),
    };
    let known = s.store.known_frontiers().unwrap();
    assert_eq!(
        (
            known.revision,
            known.membership_revision,
            known.clock_tick,
            known.clock_revision
        ),
        (4, 1, 0, 0)
    );
    let before = files(&s.scratch.db());
    let replay = Error::Supplies(SuppliesStoreError::Replay);
    assert_eq!(s.store.accept_offer(&accept, accept_replay), Err(replay));
    assert_eq!(s.store.cancel_offer(&cancel, cancel_replay), Err(replay));
    assert_eq!(files(&s.scratch.db()), before);
    for reopened in [false, true] {
        if reopened {
            drop(s.store);
            s.store = TradeStore::open_accepting(s.scratch.db(), &s.f.policy, known).unwrap();
        }
        let accepted_won = winner.kind == TradeFinalKind::Accepted;
        if accepted_won {
            assert_eq!(
                s.f.balance(&mut s.store, s.f.maker(), s.f.g),
                snapshot([17, 0, 0, 0, 25, 0], 0, 8)
            );
            assert_eq!(
                s.f.balance(&mut s.store, s.f.taker(), s.f.g),
                snapshot([8, 0, 0, 0, 0, 0], 8, 0)
            );
            assert_eq!(
                s.f.balance(&mut s.store, s.f.taker(), s.f.w),
                snapshot([5, 0, 0, 0, 10, 0], 0, 5)
            );
            assert_eq!(
                s.f.balance(&mut s.store, s.f.maker(), s.f.w),
                snapshot([5, 0, 0, 0, 0, 0], 5, 0)
            );
            let mut alias = accept;
            alias.request = RequestId::from_bytes([96; 16]);
            for value in [accept, alias] {
                let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&value));
                assert_eq!(s.store.accept_offer(&value, proof), Ok(accepted));
            }
        } else {
            assert_eq!(
                s.f.balance(&mut s.store, s.f.maker(), s.f.g),
                snapshot([25, 0, 0, 0, 25, 0], 0, 0)
            );
            assert_eq!(
                s.f.balance(&mut s.store, s.f.taker(), s.f.w),
                snapshot([10, 0, 0, 0, 10, 0], 0, 0)
            );
            assert_eq!(
                s.f.balance(&mut s.store, s.f.taker(), s.f.g),
                Default::default()
            );
            assert_eq!(
                s.f.balance(&mut s.store, s.f.maker(), s.f.w),
                Default::default()
            );
            let mut alias = cancel;
            alias.request = RequestId::from_bytes([116; 16]);
            for value in [cancel, alias] {
                let proof = s.f.attempt(&mut s.store, TradeChallenge::Cancel(&value));
                assert_eq!(s.store.cancel_offer(&value, proof), Ok(cancelled));
            }
        }
        for actor in [s.f.maker(), s.f.taker()] {
            assert_eq!(
                s.status(actor, accept.request),
                if accepted_won {
                    Some(TradeRequestOutcome::Accepted(accepted))
                } else {
                    None
                }
            );
            assert_eq!(
                s.status(actor, cancel.request),
                if accepted_won {
                    None
                } else {
                    Some(TradeRequestOutcome::Cancelled(cancelled))
                }
            );
        }
        s.closed_and_outbox(winner, &[winner]);
        assert_eq!(s.store.known_frontiers().unwrap(), known);
    }
}
