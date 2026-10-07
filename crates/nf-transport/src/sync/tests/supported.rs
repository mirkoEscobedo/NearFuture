use super::super::*;
use crate::records::PeerContext;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;
#[test]
fn supported_hello_has_exact_independent_header_and_body() {
    let pins = ExpectedProfilePins {
        implementation: [0x30; 32],
        schema: [0x31; 32],
    };
    let limits = SyncLimits::default();
    let context = PeerContext {
        session: [0; 16],
        scope: Scope {
            universe: UniverseId::from_bytes([6; 16]),
            history: HistoryId::from_bytes([7; 16]),
        },
        ruleset: [8; 32],
        content: [9; 32],
    };
    let record = SyncRecord {
        lane: SyncLane::Control,
        context,
        body: SyncBody::Hello(SyncHello {
            account: AccountId::from_bytes([1; 16]),
            device: DeviceId::from_bytes([2; 16]),
            nonce: [10; 32],
            required: 1,
            optional: 0,
            offered: limits,
            pins,
        }),
    };
    let policy = SyncWirePolicy {
        lane: SyncLane::Control,
        limits,
        pins,
    };
    let bytes = encode_body(&record, policy).expect("supported Hello must encode");
    assert_eq!(bytes.len(), 291);
    assert_eq!(&bytes[..14], b"NF-SYNC-1\0\x01\x00\x01\x01");
    assert_eq!(&bytes[126..159], &[&[1][..], &[1; 16], &[2; 16]].concat());
    assert_eq!(decode_body(&bytes, policy).unwrap(), record);
}
