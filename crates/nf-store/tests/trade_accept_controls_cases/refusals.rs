use super::*;

#[test]
fn signed_acceptance_target_and_global_identifier_conflicts_preserve_every_effect() {
    let mut s = Scenario::new(false);
    let conflict = Error::Rejected(TradeRejection::Conflict);
    for (index, reason) in [
        (0, TradeRejection::Unauthorized),
        (1, TradeRejection::Conflict),
        (2, TradeRejection::Conflict),
        (3, TradeRejection::Conflict),
    ] {
        let mut value = s.accept;
        value.request = RequestId::from_bytes([130 + index; 16]);
        value.operation = OperationId::from_bytes([140 + index; 16]);
        match index {
            0 => {
                value.actor = s.f.maker();
                value.device = s.f.supplies.beneficiary.device;
            }
            1 => value.version = 2,
            2 => value.digest[0] ^= 1,
            3 => value.offer = OfferId::from_bytes([99; 16]),
            _ => unreachable!(),
        }
        s.refuse(value, Error::Rejected(reason), None);
    }
    let mut request_collision = s.accept;
    request_collision.request = s.reserve.request;
    request_collision.operation = OperationId::from_bytes([150; 16]);
    s.refuse(
        request_collision,
        conflict,
        Some(TradeRequestOutcome::Reserved {
            operation: OperationId::from_bytes([85; 16]),
            offer: s.reserved,
        }),
    );
    for operation in [s.reserve.operation, OperationId::from_bytes([81; 16])] {
        let mut collision = s.accept;
        collision.request =
            RequestId::from_bytes([151 + u8::from(operation == s.reserve.operation); 16]);
        collision.operation = operation;
        s.refuse(collision, conflict, None);
    }
    assert_eq!(
        s.f.balance(&mut s.store, s.f.maker(), s.f.g),
        snapshot([17, 8, 0, 0, 25, 0], 0, 0)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.taker(), s.f.w),
        snapshot([5, 5, 0, 0, 10, 0], 0, 0)
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.taker(), s.f.g),
        Default::default()
    );
    assert_eq!(
        s.f.balance(&mut s.store, s.f.maker(), s.f.w),
        Default::default()
    );
    let expected = s.accepted();
    let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&s.accept));
    assert_eq!(s.store.accept_offer(&s.accept, proof), Ok(expected));
    let known = s.store.known_frontiers().unwrap();
    assert_eq!(known.revision, 4);
    drop(s.store);
    s.store = TradeStore::open_accepting(s.scratch.db(), &s.f.policy, known).unwrap();
    let mut late = s.accept;
    late.request = RequestId::from_bytes([160; 16]);
    late.operation = OperationId::from_bytes([161; 16]);
    s.refuse(late, conflict, None);
    let mut exact_request_collision = s.accept;
    exact_request_collision.operation = OperationId::from_bytes([162; 16]);
    s.refuse(
        exact_request_collision,
        conflict,
        Some(TradeRequestOutcome::Accepted(expected)),
    );
    for value in [s.accept, {
        let mut alias = s.accept;
        alias.request = RequestId::from_bytes([96; 16]);
        alias
    }] {
        let proof = s.f.attempt(&mut s.store, TradeChallenge::Accept(&value));
        assert_eq!(s.store.accept_offer(&value, proof), Ok(expected));
        assert_eq!(s.store.known_frontiers().unwrap(), known);
    }
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
    s.closed_and_outbox(expected, &[expected]);
    assert_eq!(s.store.known_frontiers().unwrap(), known);
}
