#[path = "receipt_support/corpus.rs"]
mod corpus;
use nf_contract::{canonical::binding::RequestBinding, identity::*};
use nf_transport::{auth::ServerPin, receipt::OriginalReceipt};
use sha2::{Digest, Sha256};
fn original() -> RequestBinding {
    assert_eq!(corpus::matching("record", "shape").len(), 14);
    RequestBinding {
        request_id: RequestId::from_bytes([12; 16]),
        account_id: AccountId::from_bytes([1; 16]),
        device_id: DeviceId::from_bytes([2; 16]),
        universe_id: UniverseId::from_bytes([6; 16]),
        history_id: HistoryId::from_bytes([7; 16]),
        operation_kind: 3,
        payload_digest: Sha256::digest(corpus::row("value", "full-profile1-intent")).into(),
    }
}
fn pin() -> ServerPin {
    ServerPin {
        peer: libp2p::PeerId::from_bytes(&corpus::row("value", "server-peer")).unwrap(),
        account: AccountId::from_bytes([3; 16]),
        device: DeviceId::from_bytes([4; 16]),
        minimum_membership: 11,
    }
}
#[test]
fn original_descriptor_commits_full_canonical_intent_and_refuses_missing_scope() {
    let original = original();
    assert_eq!(
        original.canonical_bytes().to_vec(),
        corpus::row("value", "original-binding")
    );
    let descriptor =
        OriginalReceipt::new(OperationId::from_bytes([13; 16]), original, pin()).unwrap();
    assert_eq!(descriptor.binding_digest(), original.digest());
    assert_eq!(descriptor.operation(), OperationId::from_bytes([13; 16]));
    assert_eq!(descriptor.request(), original.request_id);
    assert_eq!(descriptor.source().peer, pin().peer);
    let mut changed = original;
    changed.payload_digest[0] ^= 1;
    assert_ne!(
        descriptor.binding_digest(),
        OriginalReceipt::new(descriptor.operation(), changed, pin())
            .unwrap()
            .binding_digest()
    );
    let mut empty_scope = original;
    empty_scope.history_id = HistoryId::from_bytes([0; 16]);
    assert!(
        OriginalReceipt::new(descriptor.operation(), empty_scope, pin()).is_err(),
        "a zero history must not enter immutable recovery data"
    );
}
