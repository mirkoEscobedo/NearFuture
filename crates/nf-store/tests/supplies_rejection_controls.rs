mod supplies_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_identity::model::{IdentityError, Scope};
use nf_kernel::supplies::{
    Burn, BurnReason, RejectedOutcome, RequestOutcome, RequestRejection, Reserve, StatusQuery,
    SuppliesRejection,
};
use nf_store::supplies::{
    ChallengeRequest, ProofAttempt, SuppliesStore, SuppliesStoreError as Error,
};
use supplies_support::{Fixture, Scratch};
fn reserve(f: &Fixture) -> Reserve {
    let q = f.query();
    Reserve {
        request: RequestId::from_bytes([70; 16]),
        reservation: OperationId::from_bytes([71; 16]),
        actor: q.actor,
        device: q.device,
        owner: q.owner,
        universe: q.universe,
        history: q.history,
        policy: f.issuance().policy,
        content: q.content,
        origin: q.origin,
        amount: 8,
    }
}
fn status(f: &Fixture, request: RequestId) -> StatusQuery {
    let q = f.query();
    StatusQuery {
        actor: q.actor,
        device: q.device,
        owner: q.owner,
        universe: q.universe,
        history: q.history,
        request,
    }
}
#[test]
fn failed_authentication_and_admission_cannot_bind_a_business_refusal() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let original = reserve(&f);
    let before = store.known_frontiers().unwrap();
    for case in 0..6 {
        let mut value = original;
        let expected = match case {
            0 => Error::Identity(IdentityError::Signature),
            1 => {
                value.history = nf_contract::identity::HistoryId::from_bytes([99; 16]);
                Error::Rejected(SuppliesRejection::Scope)
            }
            2 => {
                value.policy[0] ^= 1;
                Error::Rejected(SuppliesRejection::Policy)
            }
            3 => {
                value.content = [99; 32];
                Error::Rejected(SuppliesRejection::UnsupportedContent)
            }
            4 => {
                value.actor = f.issuer.account;
                value.device = f.issuer.device;
                Error::Rejected(SuppliesRejection::Unauthorized)
            }
            _ => {
                value.amount = 0;
                Error::Rejected(SuppliesRejection::Limit)
            }
        };
        let mut proof = f.attempt(&mut store, ChallengeRequest::Reserve(&value));
        if case == 0 {
            proof.proof.signature[0] ^= 1;
        }
        assert_eq!(store.reserve(&value, proof), Err(expected));
        assert_eq!(
            store.known_frontiers().unwrap(),
            before,
            "failed authorization/admission is not a terminal business decision"
        );
        let query = status(&f, original.request);
        let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
        assert_eq!(store.status(&query, proof).unwrap(), None);
    }
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&original));
    assert_eq!(
        store.reserve(&original, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    assert_eq!(
        store.known_frontiers().unwrap().revision,
        1,
        "same IDs remain available for the first actual admitted refusal"
    );
    let mut bad = f.attempt(&mut store, ChallengeRequest::Reserve(&original));
    bad.proof.signature[0] ^= 1;
    assert_eq!(
        store.reserve(&original, bad),
        Err(Error::Identity(IdentityError::Signature)),
        "dedupe never bypasses authentication"
    );
    assert_eq!(store.known_frontiers().unwrap().revision, 1);
}
#[test]
fn immutable_rejected_operation_and_request_terms_conflict_without_rebinding() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let original = reserve(&f);
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&original));
    assert_eq!(
        store.reserve(&original, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    let known = store.known_frontiers().unwrap();
    assert_eq!(known.revision, 1);
    let mut changed = original;
    changed.amount = 9;
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&changed));
    assert_eq!(
        store.reserve(&changed, proof),
        Err(Error::Rejected(SuppliesRejection::Conflict))
    );
    changed.request = RequestId::from_bytes([72; 16]);
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&changed));
    assert_eq!(
        store.reserve(&changed, proof),
        Err(Error::Rejected(SuppliesRejection::Conflict))
    );
    let issue = f.issuance();
    let collision = Burn {
        request: RequestId::from_bytes([73; 16]),
        burn: original.reservation,
        actor: issue.actor,
        device: issue.device,
        owner: issue.beneficiary,
        universe: issue.universe,
        history: issue.history,
        policy: issue.policy,
        content: issue.content,
        origin: issue.origin,
        reason: BurnReason::AuthorityDestruction,
        amount: 8,
    };
    let proof = f.attempt(&mut store, ChallengeRequest::Burn(&collision));
    assert_eq!(
        store.burn(&collision, proof),
        Err(Error::Rejected(SuppliesRejection::Conflict))
    );
    assert_eq!(store.known_frontiers().unwrap(), known);
    for request in [changed.request, collision.request] {
        let query = status(&f, request);
        let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
        assert_eq!(store.status(&query, proof).unwrap(), None);
    }
    let query = status(&f, original.request);
    let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
    assert_eq!(
        store.status(&query, proof).unwrap(),
        Some(RequestOutcome::Rejected(RejectedOutcome {
            operation: original.reservation,
            revision: 1,
            reason: RequestRejection::InsufficientAvailable
        }))
    );
    let q = f.query();
    let proof = f.attempt(&mut store, ChallengeRequest::Balance(&q));
    let b = store.balance(&q, proof).unwrap();
    assert_eq!(
        (
            b.available,
            b.reserved,
            b.pending,
            b.externalized,
            b.minted,
            b.burned
        ),
        (0, 0, 0, 0, 0, 0)
    );
    assert_eq!(
        known.scope,
        Scope {
            universe: q.universe,
            history: q.history
        }
    );
}
#[test]
fn expired_and_replayed_tickets_are_not_recorded_as_business_refusals() {
    let scratch = Scratch::new();
    let f = Fixture::new();
    let mut store = SuppliesStore::create(scratch.db(), &f.policy, &f.membership).unwrap();
    let original = reserve(&f);
    let empty = store.known_frontiers().unwrap();
    let expired = f.attempt(&mut store, ChallengeRequest::Reserve(&original));
    // Exercise the actual unchanged five-second authorization lifetime, with no injected clock.
    std::thread::sleep(std::time::Duration::from_millis(5100));
    assert_eq!(store.reserve(&original, expired), Err(Error::Expired));
    assert_eq!(store.known_frontiers().unwrap(), empty);
    let query = status(&f, original.request);
    let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
    assert_eq!(store.status(&query, proof).unwrap(), None);
    let proof = f.attempt(&mut store, ChallengeRequest::Reserve(&original));
    let duplicate = ProofAttempt {
        ticket: proof.ticket,
        proof: proof.proof.clone(),
    };
    assert_eq!(
        store.reserve(&original, proof),
        Err(Error::Rejected(SuppliesRejection::Limit))
    );
    assert_eq!(store.reserve(&original, duplicate), Err(Error::Replay));
    assert_eq!(store.known_frontiers().unwrap().revision, 1);
    let proof = f.attempt(&mut store, ChallengeRequest::Status(&query));
    assert_eq!(
        store.status(&query, proof).unwrap(),
        Some(RequestOutcome::Rejected(RejectedOutcome {
            operation: original.reservation,
            revision: 1,
            reason: RequestRejection::InsufficientAvailable,
        }))
    );
}
