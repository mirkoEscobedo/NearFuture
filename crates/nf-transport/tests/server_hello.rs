use nf_transport::records::{Lane, PeerBody, PeerLimits, decode_body};
#[test]
fn server_hello_has_explicit_caps_selected_limits_and_fixed_proof_width() {
    let peer = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_bytes();
    let mut b = b"NF-PEER-1\0".to_vec();
    b.extend(1u16.to_le_bytes());
    b.extend([2, 1]);
    for v in [9u8, 1, 2] {
        b.extend([v; 16]);
    }
    for v in [3u8, 4] {
        b.extend([v; 32]);
    }
    b.extend([8; 32]);
    for v in [3u32, 1] {
        b.extend(v.to_le_bytes());
    }
    for _ in 0..2 {
        for v in [4096u32, 9216, 8192, 65536, 131072] {
            b.extend(v.to_le_bytes());
        }
        for v in [16u16, 4, 8] {
            b.extend(v.to_le_bytes());
        }
    }
    for v in [1u8, 2, 5, 6] {
        b.extend([v; 16]);
    }
    b.extend(7u64.to_le_bytes());
    b.push(peer.len() as u8);
    b.extend(&peer);
    b.resize(b.len() + 128 - peer.len(), 0);
    b.extend([10; 32]);
    b.extend([11; 64]);
    assert_eq!(b.len(), 515);
    let record = decode_body(&b, Lane::Control, PeerLimits::default()).unwrap();
    let PeerBody::ServerHello {
        nonce,
        available,
        selected_caps,
        server_limits,
        selected,
        proof,
    } = record.body
    else {
        panic!("server hello")
    };
    assert_eq!(nonce, [8; 32]);
    assert_eq!((available, selected_caps), (3, 1));
    assert_eq!(server_limits, selected);
    assert_eq!(proof.frontier, 7);
    b[162] = 4;
    assert!(matches!(
        decode_body(&b, Lane::Control, PeerLimits::default()),
        Err(nf_transport::PeerError::Unsupported)
    ));
}
