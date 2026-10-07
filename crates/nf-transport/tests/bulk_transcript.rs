use nf_transport::{auth::BulkTranscript, records::BulkDescriptor};
use sha2::{Digest, Sha256};
#[test]
fn bulk_transcript_binds_exact_descriptor_and_explicit_minimum() {
    let d = BulkDescriptor {
        transfer: [9; 16],
        total: 12,
        chunks: 2,
        digest: [8; 32],
    };
    let t = BulkTranscript {
        context_digest: [1; 32],
        descriptor: d,
        client_nonce: [2; 32],
        server_nonce: [3; 32],
        frontier: 7,
        minimum_membership: 6,
    };
    let mut descriptor = Vec::new();
    descriptor.extend([9; 16]);
    descriptor.extend(12u64.to_le_bytes());
    descriptor.extend(2u16.to_le_bytes());
    descriptor.extend([8; 32]);
    assert_eq!(descriptor.len(), 58);
    let mut expected = Vec::new();
    expected.extend(b"NF-PEER-BULK-1\0");
    expected.push(1);
    expected.extend([1; 32]);
    expected.extend([9; 16]);
    expected.extend([2; 32]);
    expected.extend([3; 32]);
    expected.extend(7u64.to_le_bytes());
    expected.extend(6u64.to_le_bytes());
    expected.extend(Sha256::digest(descriptor));
    expected.extend([0; 32]);
    assert_eq!(expected.len(), 208);
    assert_eq!(
        t.challenge(1, [0; 32]).unwrap(),
        <[u8; 32]>::from(Sha256::digest(expected))
    );
    let mut changed = t;
    changed.minimum_membership = 5;
    assert_ne!(
        t.challenge(1, [0; 32]).unwrap(),
        changed.challenge(1, [0; 32]).unwrap()
    );
}
