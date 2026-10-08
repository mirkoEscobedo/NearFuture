mod outsider;
use crate::{Error, Scratch, TradeChallenge, TradeFixture, TradeRejection, TradeRequestOutcome};
use nf_identity::model::IdentityError;
use nf_store::supplies::{ProofAttempt, SuppliesStoreError};
fn signature_error() -> Error {
    Error::Supplies(SuppliesStoreError::Identity(IdentityError::Signature))
}
#[test]
fn either_bad_signature_cannot_create_or_bypass_a_durable_refusal() {
    for corrupt_maker in [true, false] {
        let scratch = Scratch::new();
        let f = TradeFixture::new();
        let mut store = f.funded(&scratch);
        let value = f.reserve_offer();
        let before = store.known_frontiers().unwrap();
        for recorded in [false, true] {
            let mut maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
            let mut taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
            if corrupt_maker {
                maker.proof.signature[0] ^= 1;
            } else {
                taker.proof.signature[0] ^= 1;
            }
            assert_eq!(
                store.reserve_offer(&value, maker, taker),
                Err(signature_error())
            );
            if !recorded {
                assert_eq!(store.known_frontiers().unwrap(), before);
                for party in [f.maker(), f.taker()] {
                    assert_eq!(f.request_status(&mut store, party, value.request), None);
                }
                assert_eq!(
                    f.refuse(&mut store, &value),
                    Err(Error::Rejected(TradeRejection::InsufficientAvailable))
                );
            } else {
                assert_eq!(store.known_frontiers().unwrap().revision, 3);
                for party in [f.maker(), f.taker()] {
                    assert_eq!(
                        f.request_status(&mut store, party, value.request),
                        Some(TradeRequestOutcome::Rejected {
                            operation: value.operation,
                            revision: 3,
                            reason: TradeRejection::InsufficientAvailable
                        })
                    );
                }
            }
        }
        assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
        assert_eq!(f.balance(&mut store, f.taker()), [4, 0, 0, 0, 4, 0]);
    }
}
#[test]
fn signed_invalid_admission_does_not_consume_the_first_business_decision() {
    for case in 0..7 {
        let scratch = Scratch::new();
        let f = TradeFixture::new();
        let mut store = f.funded(&scratch);
        let original = f.reserve_offer();
        let mut changed = original;
        let reason = match case {
            0 => {
                changed.terms.history = nf_contract::identity::HistoryId::from_bytes([99; 16]);
                TradeRejection::Scope
            }
            1 => {
                changed.terms.policy[0] ^= 1;
                TradeRejection::Policy
            }
            2 => {
                changed.terms.give.content = [99; 32];
                TradeRejection::UnsupportedContent
            }
            3 => {
                changed.terms.want.origin.lineage = [99; 32];
                TradeRejection::Policy
            }
            4 => {
                changed.terms.give.amount = 0;
                TradeRejection::Limit
            }
            5 => {
                changed.terms.expires_at = 0;
                TradeRejection::Limit
            }
            _ => {
                changed.terms.taker = changed.terms.maker;
                changed.taker_device = changed.maker_device;
                TradeRejection::Unauthorized
            }
        };
        let before = store.known_frontiers().unwrap();
        assert_eq!(f.refuse(&mut store, &changed), Err(Error::Rejected(reason)));
        assert_eq!(store.known_frontiers().unwrap(), before);
        for party in [f.maker(), f.taker()] {
            assert_eq!(f.request_status(&mut store, party, original.request), None);
        }
        assert_eq!(
            f.refuse(&mut store, &original),
            Err(Error::Rejected(TradeRejection::InsufficientAvailable))
        );
        assert_eq!(store.known_frontiers().unwrap().revision, 3);
    }
}
#[test]
fn proof_purposes_and_consumed_tickets_cannot_bind_or_repeat_a_refusal() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let mut store = f.funded(&scratch);
    let value = f.reserve_offer();
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
    assert_eq!(
        store.reserve_offer(&value, taker, maker),
        Err(Error::Supplies(SuppliesStoreError::Identity(
            IdentityError::Frontier
        )))
    );
    assert_eq!(store.known_frontiers().unwrap().revision, 2);
    let maker = f.attempt(&mut store, TradeChallenge::ReserveMaker(&value));
    let taker = f.attempt(&mut store, TradeChallenge::ReserveTaker(&value));
    let duplicate_maker = ProofAttempt {
        ticket: maker.ticket,
        proof: maker.proof.clone(),
    };
    let duplicate_taker = ProofAttempt {
        ticket: taker.ticket,
        proof: taker.proof.clone(),
    };
    assert_eq!(
        store.reserve_offer(&value, maker, taker),
        Err(Error::Rejected(TradeRejection::InsufficientAvailable))
    );
    assert_eq!(
        store.reserve_offer(&value, duplicate_maker, duplicate_taker),
        Err(Error::Supplies(SuppliesStoreError::Replay))
    );
    assert_eq!(store.known_frontiers().unwrap().revision, 3);
    assert_eq!(
        f.request_status(&mut store, f.maker(), value.request),
        Some(TradeRequestOutcome::Rejected {
            operation: value.operation,
            revision: 3,
            reason: TradeRejection::InsufficientAvailable
        })
    );
}
