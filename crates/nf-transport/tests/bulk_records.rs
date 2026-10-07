use nf_contract::identity::{HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_transport::{PeerError, records::*};
#[test]
fn bulk_begin_has_explicit_minimum_and_chunk_limit_before_payload() {
    let mut b = Vec::new();
    b.extend(b"NF-PEER-1\0");
    b.extend(1u16.to_le_bytes());
    b.extend([10, 2]);
    b.extend([1; 16]);
    b.extend([2; 16]);
    b.extend([3; 16]);
    b.extend([4; 32]);
    b.extend([5; 32]);
    b.extend([9; 16]);
    b.extend(12u64.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend([8; 32]);
    b.extend([7; 32]);
    b.extend(6u64.to_le_bytes());
    let r = decode_body(&b, Lane::Bulk, PeerLimits::default()).unwrap();
    assert_eq!(
        r.body,
        PeerBody::BeginBulk {
            descriptor: BulkDescriptor {
                transfer: [9; 16],
                total: 12,
                chunks: 2,
                digest: [8; 32]
            },
            nonce: [7; 32],
            minimum_membership: 6
        }
    );
    assert_eq!(
        encode_body(&r, Lane::Bulk, PeerLimits::default()).unwrap(),
        b
    );
    b.truncate(126);
    b[12] = 13;
    b.extend([9; 16]);
    b.extend(0u16.to_le_bytes());
    b.extend(8193u16.to_le_bytes());
    assert_eq!(
        decode_body(&b, Lane::Bulk, PeerLimits::default()),
        Err(PeerError::Limit)
    );
    let oversized = PeerRecord {
        context: PeerContext {
            session: [1; 16],
            scope: Scope {
                universe: UniverseId::from_bytes([2; 16]),
                history: HistoryId::from_bytes([3; 16]),
            },
            ruleset: [4; 32],
            content: [5; 32],
        },
        body: PeerBody::BulkChunk {
            transfer: [9; 16],
            index: 0,
            bytes: vec![0; 8193],
        },
    };
    assert_eq!(
        encode_body(&oversized, Lane::Bulk, PeerLimits::default()),
        Err(PeerError::Limit)
    );
}
