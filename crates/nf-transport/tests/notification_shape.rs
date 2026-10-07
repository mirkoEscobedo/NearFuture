use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_transport::notification::{
    NotifyBody, NotifyContext, NotifyLimits, NotifyRecord, PROTOCOL, decode_body, encode_body,
};

#[test]
fn notification_hello_roundtrips_in_its_exact_namespace() {
    let record = NotifyRecord {
        context: NotifyContext {
            session: [0; 16],
            scope: Scope {
                universe: UniverseId::from_bytes([1; 16]),
                history: HistoryId::from_bytes([2; 16]),
            },
            ruleset: [3; 32],
            content: [4; 32],
        },
        body: NotifyBody::Hello {
            account: AccountId::from_bytes([5; 16]),
            device: DeviceId::from_bytes([6; 16]),
            nonce: [7; 32],
            required: 1,
            optional: 0,
            offered: NotifyLimits::default(),
        },
    };
    let encoded =
        encode_body(&record, PROTOCOL, NotifyLimits::default()).expect("valid notification Hello");
    assert_eq!(encoded.len(), 214);
    assert_eq!(
        decode_body(&encoded, PROTOCOL, NotifyLimits::default()).unwrap(),
        record
    );
}

#[test]
fn immutable_request_subscription_roundtrips_with_its_closed_selector() {
    use nf_contract::identity::{OperationId, RequestId};
    use nf_transport::notification::NotifySelector;
    let record = NotifyRecord {
        context: NotifyContext {
            session: [9; 16],
            scope: Scope {
                universe: UniverseId::from_bytes([1; 16]),
                history: HistoryId::from_bytes([2; 16]),
            },
            ruleset: [3; 32],
            content: [4; 32],
        },
        body: NotifyBody::BeginSubscribe {
            subscription: [10; 16],
            selector: NotifySelector {
                request: RequestId::from_bytes([11; 16]),
                operation: OperationId::from_bytes([12; 16]),
                binding: [13; 32],
            },
            nonce: [14; 32],
            minimum_membership: 3,
            lifetime: 30,
        },
    };
    let encoded =
        encode_body(&record, PROTOCOL, NotifyLimits::default()).expect("valid bound subscription");
    assert_eq!(encoded.len(), 251);
    assert_eq!(encoded[144], 1, "closed receipt-available topic");
    assert_eq!(
        decode_body(&encoded, PROTOCOL, NotifyLimits::default()).unwrap(),
        record
    );
}

#[test]
fn notice_preserves_only_immutable_request_and_ephemeral_hint_fields() {
    use nf_contract::identity::{OperationId, RequestId};
    use nf_identity::model::DeviceProof;
    use nf_transport::notification::NotifySelector;
    let scope = Scope {
        universe: UniverseId::from_bytes([1; 16]),
        history: HistoryId::from_bytes([2; 16]),
    };
    let peer = libp2p::identity::Keypair::ed25519_from_bytes([31; 32])
        .unwrap()
        .public()
        .to_peer_id()
        .to_bytes();
    // Shape-only signature bytes deliberately provide no authentication claim.
    let proof = DeviceProof {
        scope,
        account: AccountId::from_bytes([5; 16]),
        device: DeviceId::from_bytes([6; 16]),
        frontier: 3,
        peer,
        challenge: [17; 32],
        signature: [0; 64],
    };
    let record = NotifyRecord {
        context: NotifyContext {
            session: [9; 16],
            scope,
            ruleset: [3; 32],
            content: [4; 32],
        },
        body: NotifyBody::Notice {
            subscription: [10; 16],
            sequence: 1,
            selector: NotifySelector {
                request: RequestId::from_bytes([11; 16]),
                operation: OperationId::from_bytes([12; 16]),
                binding: [13; 32],
            },
            nonce: [14; 32],
            proof,
        },
    };
    let encoded =
        encode_body(&record, PROTOCOL, NotifyLimits::default()).expect("valid dirty hint shape");
    assert_eq!(encoded.len(), 545);
    assert_eq!(
        decode_body(&encoded, PROTOCOL, NotifyLimits::default()).unwrap(),
        record
    );
}
