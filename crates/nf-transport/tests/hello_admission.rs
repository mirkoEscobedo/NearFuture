use nf_transport::records::{Lane, PeerBody, PeerLimits, decode_body};
fn hello() -> Vec<u8> {
    let mut b = b"NF-PEER-1\0".to_vec();
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&[1, 1]);
    b.extend_from_slice(&[0; 16]);
    b.extend_from_slice(&[1; 16]);
    b.extend_from_slice(&[2; 16]);
    b.extend_from_slice(&[3; 32]);
    b.extend_from_slice(&[4; 32]);
    b.extend_from_slice(&[5; 16]);
    b.extend_from_slice(&[6; 16]);
    b.extend_from_slice(&[7; 32]);
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&2u32.to_le_bytes());
    for v in [4096u32, 9216, 8192, 65536, 131072] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for v in [16u16, 4, 8] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}
#[test]
fn initial_hello_has_closed_capabilities_and_hard_limits_before_activation() {
    let wire = hello();
    assert_eq!(wire.len(), 224);
    let body = decode_body(&wire, Lane::Control, PeerLimits::default())
        .unwrap()
        .body;
    let PeerBody::Hello {
        account,
        device,
        nonce,
        required,
        optional,
        offered,
    } = body
    else {
        panic!("hello")
    };
    assert_eq!(account.as_bytes(), &[5; 16]);
    assert_eq!(device.as_bytes(), &[6; 16]);
    assert_eq!(nonce, [7; 32]);
    assert_eq!((required, optional), (1, 2));
    assert_eq!(offered, PeerLimits::default());
    let mut unknown = wire.clone();
    unknown[190] = 4;
    assert!(matches!(
        decode_body(&unknown, Lane::Control, PeerLimits::default()),
        Err(nf_transport::PeerError::Unsupported)
    ));
    let mut over = wire.clone();
    over[206..210].copy_from_slice(&8193u32.to_le_bytes());
    assert!(matches!(
        decode_body(&over, Lane::Control, PeerLimits::default()),
        Err(nf_transport::PeerError::Limit)
    ));
}
