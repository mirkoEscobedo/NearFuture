use super::*;

#[test]
fn same_origin_acceptance_coalesces_four_legs_into_exactly_two_keys() {
    let mut s = Scenario::new(true);
    assert_eq!(
        s.f.balance(&mut s.store, s.f.maker(), s.f.g),
        snapshot([17, 8, 0, 0, 25, 0], 0, 0)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.taker(), s.f.g),
        snapshot([5, 5, 0, 0, 10, 0], 0, 0)
    );
    let expected = s.accepted();
    let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&s.accept));
    assert_eq!(s.store.accept_offer(&s.accept, proof), Ok(expected));
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
    for reopened in [false, true] {
        if reopened {
            drop(s.store);
            let sql = rusqlite::Connection::open(s.scratch.db()).unwrap();
            for table in ["supplies_balances", "trade_flows"] {
                let count: u64 = sql
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .unwrap();
                assert_eq!(
                    count, 2,
                    "coalesced cache has precisely the two original account/asset keys"
                );
            }
            drop(sql);
            s.store = TradeStore::open_accepting(s.scratch.db(), &s.f.policy, known).unwrap();
        }
        assert_eq!(
            s.f.balance(&mut s.store, s.f.maker(), s.f.g),
            snapshot([22, 0, 0, 0, 25, 0], 5, 8)
        );
        assert_eq!(
            s.f.balance(&mut s.store, s.f.taker(), s.f.g),
            snapshot([13, 0, 0, 0, 10, 0], 8, 5)
        );
        let mut alias = s.accept;
        alias.request = RequestId::from_bytes([96; 16]);
        for value in [s.accept, alias] {
            let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&value));
            assert_eq!(s.store.accept_offer(&value, proof), Ok(expected));
            assert_eq!(s.store.known_frontiers().unwrap(), known);
            for actor in [s.f.maker(), s.f.taker()] {
                assert_eq!(
                    s.status(actor, value.request),
                    Some(TradeRequestOutcome::Accepted(expected))
                );
            }
        }
        s.closed_and_outbox(expected, &[expected]);
        assert_eq!(s.store.known_frontiers().unwrap(), known);
    }
}
