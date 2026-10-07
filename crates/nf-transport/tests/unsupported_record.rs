use nf_transport::records::{Lane, PeerBody, PeerLimits, UnsupportedReason, decode_body};
#[test]
fn unsupported_is_closed_authenticated_data_and_never_arbitrary_error_text() {
    let p = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_bytes();
    let mut b = b"NF-PEER-1\0".to_vec();
    b.extend(1u16.to_le_bytes());
    b.extend([9, 1]);
    for v in [9u8, 1, 2] {
        b.extend([v; 16]);
    }
    for v in [3u8, 4] {
        b.extend([v; 32]);
    }
    b.extend([5; 16]);
    b.push(1);
    for v in [1u8, 2, 10, 11] {
        b.extend([v; 16]);
    }
    b.extend(12u64.to_le_bytes());
    b.push(p.len() as u8);
    b.extend(&p);
    b.resize(b.len() + 128 - p.len(), 0);
    b.extend([13; 32]);
    b.extend([14; 64]);
    assert_eq!(b.len(), 440);
    let r = decode_body(&b, Lane::Control, PeerLimits::default()).unwrap();
    assert!(matches!(
        r.body,
        PeerBody::Unsupported {
            reason: UnsupportedReason::KernelOutcome,
            ..
        }
    ));
    b[142] = 9;
    assert_eq!(
        decode_body(&b, Lane::Control, PeerLimits::default()),
        Err(nf_transport::PeerError::Malformed)
    );
}
