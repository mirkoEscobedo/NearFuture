use nf_transport::PeerError;
use nf_transport::records::{Lane, PeerBody, PeerLimits, decode_body, encode_body};
fn proof_frame(kind: u8) -> Vec<u8> {
    let peer = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_bytes();
    let mut b = b"NF-PEER-1\0".to_vec();
    b.extend(1u16.to_le_bytes());
    b.extend([kind, 1]);
    b.extend([9; 16]);
    b.extend([1; 16]);
    b.extend([2; 16]);
    b.extend([3; 32]);
    b.extend([4; 32]);
    b.extend([1; 16]);
    b.extend([2; 16]);
    b.extend([5; 16]);
    b.extend([6; 16]);
    b.extend(7u64.to_le_bytes());
    b.push(peer.len() as u8);
    b.extend(peer.iter());
    b.resize(b.len() + 128 - peer.len(), 0);
    b.extend([8; 32]);
    b.extend([10; 64]);
    b
}
#[test]
fn proof_shape_is_fixed_canonical_bounded_and_scope_bound() {
    let bytes = proof_frame(3);
    assert_eq!(bytes.len(), 423);
    let record = decode_body(&bytes, Lane::Control, PeerLimits::default()).unwrap();
    let PeerBody::ClientProof(proof) = &record.body else {
        panic!("proof")
    };
    assert_eq!(proof.frontier, 7);
    assert_eq!(proof.challenge, [8; 32]);
    assert_eq!(
        encode_body(&record, Lane::Control, PeerLimits::default()).unwrap(),
        bytes
    );
    let mut padding = bytes.clone();
    padding[326] = 1;
    assert_eq!(
        decode_body(&padding, Lane::Control, PeerLimits::default()),
        Err(PeerError::Malformed)
    );
    let mut scope = bytes;
    scope[126] = 11;
    assert_eq!(
        decode_body(&scope, Lane::Control, PeerLimits::default()),
        Err(PeerError::Scope)
    );
}
