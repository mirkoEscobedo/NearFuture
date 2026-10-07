use nf_contract::identity::RequestId;
use nf_transport::auth::QueryTranscript;
use sha2::{Digest, Sha256};
#[test]
fn fresh_query_proof_and_reply_bind_request_nonces_frontiers_and_exact_result_digest() {
    let q = QueryTranscript {
        context_digest: [1; 32],
        request: RequestId::from_bytes([2; 16]),
        client_nonce: [3; 32],
        server_nonce: [4; 32],
        frontier: 5,
        minimum_membership: 4,
    };
    let mut b = b"NF-PEER-QUERY-1\0".to_vec();
    b.push(1);
    b.extend([1; 32]);
    b.extend([2; 16]);
    b.extend([3; 32]);
    b.extend([4; 32]);
    b.extend(5u64.to_le_bytes());
    b.extend(4u64.to_le_bytes());
    b.extend([0; 32]);
    assert_eq!(b.len(), 177);
    assert_eq!(
        q.challenge(1, [0; 32]).unwrap(),
        <[u8; 32]>::from(Sha256::digest(&b))
    );
    assert!(q.challenge(1, [9; 32]).is_err());
    assert_ne!(
        q.challenge(2, [8; 32]).unwrap(),
        q.challenge(2, [9; 32]).unwrap()
    );
    let mut invalid = q;
    invalid.minimum_membership = 6;
    assert!(invalid.challenge(1, [0; 32]).is_err());
}
