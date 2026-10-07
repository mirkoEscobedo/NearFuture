use nf_transport::PeerError;
use nf_transport::records::{
    Lane, PeerBody, PeerLimits, RetainedPhase, decode_body, reply_prefix_digest,
};
use sha2::{Digest, Sha256};
fn status() -> Vec<u8> {
    let p = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_bytes();
    let mut b = b"NF-PEER-1\0".to_vec();
    b.extend(1u16.to_le_bytes());
    b.extend([8, 1]);
    for v in [9u8, 1, 2] {
        b.extend([v; 16]);
    }
    for v in [3u8, 4] {
        b.extend([v; 32]);
    }
    b.extend([5; 16]);
    b.extend([6; 16]);
    b.extend([7; 32]);
    b.extend([2, 0]);
    b.extend(0u64.to_le_bytes());
    b.push(0);
    for v in [1u8, 2, 10, 11] {
        b.extend([v; 16]);
    }
    b.extend(12u64.to_le_bytes());
    b.push(p.len() as u8);
    b.extend(&p);
    b.resize(b.len() + 128 - p.len(), 0);
    b.extend([13; 32]);
    b.extend([14; 64]);
    b
}
#[test]
fn retained_status_is_closed_and_exact_header_and_body_prefix_is_signed() {
    let b = status();
    assert_eq!(b.len(), 498);
    let r = decode_body(&b, Lane::Control, PeerLimits::default()).unwrap();
    let PeerBody::RetainedStatus { phase, .. } = &r.body else {
        panic!("status")
    };
    assert!(matches!(phase, RetainedPhase::Pending { .. }));
    assert_eq!(
        reply_prefix_digest(&r, Lane::Control, PeerLimits::default()).unwrap(),
        <[u8; 32]>::from(Sha256::digest(&b[..201]))
    );
    let mut unknown = b.clone();
    unknown[190] = 4;
    assert_eq!(
        decode_body(&unknown, Lane::Control, PeerLimits::default()),
        Err(PeerError::Malformed)
    );
    let mut false_sequence = b;
    false_sequence[192] = 1;
    assert_eq!(
        decode_body(&false_sequence, Lane::Control, PeerLimits::default()),
        Err(PeerError::Malformed)
    );
}
