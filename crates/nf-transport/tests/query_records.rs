use nf_transport::records::{Lane, PeerBody, PeerLimits, decode_body};
#[test]
fn query_challenge_fields_are_exact_and_lane_cannot_be_selected_by_record() {
    let mut b = b"NF-PEER-1\0".to_vec();
    b.extend(1u16.to_le_bytes());
    b.extend([6, 1]);
    for v in [9u8, 1, 2] {
        b.extend([v; 16]);
    }
    for v in [3u8, 4] {
        b.extend([v; 32]);
    }
    b.extend([5; 16]);
    b.extend([6; 32]);
    b.extend([7; 32]);
    b.extend(8u64.to_le_bytes());
    b.extend([10; 32]);
    assert_eq!(b.len(), 246);
    let r = decode_body(&b, Lane::Control, PeerLimits::default()).unwrap();
    let PeerBody::QueryChallenge {
        request,
        client_nonce,
        server_nonce,
        frontier,
        challenge,
    } = r.body
    else {
        panic!("challenge")
    };
    assert_eq!(request.as_bytes(), &[5; 16]);
    assert_eq!(client_nonce, [6; 32]);
    assert_eq!(server_nonce, [7; 32]);
    assert_eq!(frontier, 8);
    assert_eq!(challenge, [10; 32]);
    assert_eq!(
        decode_body(&b, Lane::Bulk, PeerLimits::default()),
        Err(nf_transport::PeerError::Unauthorized)
    );
}
