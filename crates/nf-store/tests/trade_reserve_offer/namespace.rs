use crate::{Error, Scratch, TradeChallenge, TradeFixture, TradeRejection, TradeRequestOutcome};
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{IssuanceOutcome, RequestOutcome, SuppliesRejection};
use nf_store::supplies::SuppliesStoreError;
#[test]
fn inherited_issuance_and_joint_refusal_share_request_and_operation_ids() {
    for collide_request in [true, false] {
        let scratch = Scratch::new();
        let f = TradeFixture::new();
        let mut store = f.funded(&scratch);
        let original = f.reserve_offer();
        let mut collision = original;
        if collide_request {
            collision.request = RequestId::from_bytes([80; 16]);
        } else {
            collision.operation = OperationId::from_bytes([81; 16]);
        }
        assert_eq!(
            f.refuse(&mut store, &collision),
            Err(Error::Rejected(TradeRejection::Conflict))
        );
        assert_eq!(store.known_frontiers().unwrap().revision, 2);
        assert_eq!(
            f.request_status(&mut store, f.maker(), original.request),
            None
        );
        assert_eq!(
            f.refuse(&mut store, &original),
            Err(Error::Rejected(TradeRejection::InsufficientAvailable))
        );
        let known = store.known_frontiers().unwrap();
        let mut issue = f.supplies.issuance();
        issue.request = if collide_request {
            original.request
        } else {
            RequestId::from_bytes([98; 16])
        };
        issue.issuance = if collide_request {
            OperationId::from_bytes([99; 16])
        } else {
            original.operation
        };
        let proof = f.attempt(&mut store, TradeChallenge::Issue(&issue));
        assert_eq!(
            store.issue(&issue, proof),
            Err(Error::Supplies(SuppliesStoreError::Rejected(
                SuppliesRejection::Conflict
            )))
        );
        assert_eq!(store.known_frontiers().unwrap(), known);
        if !collide_request {
            assert_eq!(f.request_status(&mut store, f.maker(), issue.request), None);
        }
        assert_eq!(
            f.request_status(&mut store, f.maker(), RequestId::from_bytes([80; 16])),
            Some(TradeRequestOutcome::Supplies(RequestOutcome::Issued(
                IssuanceOutcome {
                    issuance: OperationId::from_bytes([81; 16]),
                    revision: 1
                }
            )))
        );
        for party in [f.maker(), f.taker()] {
            assert_eq!(
                f.request_status(&mut store, party, original.request),
                Some(TradeRequestOutcome::Rejected {
                    operation: original.operation,
                    revision: 3,
                    reason: TradeRejection::InsufficientAvailable
                })
            );
        }
        assert_eq!(f.balance(&mut store, f.maker()), [25, 0, 0, 0, 25, 0]);
        assert_eq!(f.balance(&mut store, f.taker()), [4, 0, 0, 0, 4, 0]);
    }
}
#[test]
fn admitted_changed_offer_terms_conflict_without_rebinding_a_refusal() {
    let scratch = Scratch::new();
    let mut f = TradeFixture::new();
    let mut rule = f.policy.supplies.issuers[0];
    rule.origin.lineage = [5; 32];
    f.policy.supplies.issuers.push(rule);
    f.policy.allowed_origins.push(rule.origin);
    f.supplies.policy = f.policy.supplies.clone();
    let mut store = f.funded(&scratch);
    let original = f.reserve_offer();
    assert_eq!(
        f.refuse(&mut store, &original),
        Err(Error::Rejected(TradeRejection::InsufficientAvailable))
    );
    let known = store.known_frontiers().unwrap();
    for case in 0..5 {
        let mut changed = original;
        changed.request = RequestId::from_bytes([100 + case; 16]);
        match case {
            0 => changed.terms.offer = nf_kernel::trade::OfferId::from_bytes([99; 16]),
            1 => changed.terms.version = 2,
            2 => changed.terms.give.amount = 9,
            3 => changed.terms.want.origin = rule.origin,
            _ => changed.terms.expires_at = 11,
        }
        assert_eq!(
            f.refuse(&mut store, &changed),
            Err(Error::Rejected(TradeRejection::Conflict))
        );
        assert_eq!(store.known_frontiers().unwrap(), known);
        for party in [f.maker(), f.taker()] {
            assert_eq!(f.request_status(&mut store, party, changed.request), None);
        }
    }
    let mut alias = original;
    alias.request = RequestId::from_bytes([105; 16]);
    assert_eq!(
        f.refuse(&mut store, &alias),
        Err(Error::Rejected(TradeRejection::InsufficientAvailable))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    for party in [f.maker(), f.taker()] {
        assert_eq!(
            f.request_status(&mut store, party, alias.request),
            Some(TradeRequestOutcome::Rejected {
                operation: original.operation,
                revision: 3,
                reason: TradeRejection::InsufficientAvailable
            })
        );
    }
}
