use nf_transport::records::{Lane, PeerBody, PeerLimits, decode_body};
#[test]
fn independently_written_query_is_closed_and_declared_version_precedes_body_reads() {
    let mut wire = b"NF-PEER-1\0".to_vec();
    wire.extend_from_slice(&1u16.to_le_bytes());
    wire.extend_from_slice(&[5, 1]);
    wire.extend_from_slice(&[9; 16]);
    wire.extend_from_slice(&[1; 16]);
    wire.extend_from_slice(&[2; 16]);
    wire.extend_from_slice(&[3; 32]);
    wire.extend_from_slice(&[4; 32]);
    wire.extend_from_slice(&[8; 16]);
    wire.extend_from_slice(&[10; 32]);
    wire.extend_from_slice(&7u64.to_le_bytes());
    let record = decode_body(&wire, Lane::Control, PeerLimits::default()).unwrap();
    let PeerBody::BeginQuery {
        request,
        nonce,
        minimum_membership,
    } = record.body
    else {
        panic!("query")
    };
    assert_eq!(request.as_bytes(), &[8; 16]);
    assert_eq!(nonce, [10; 32]);
    assert_eq!(minimum_membership, 7);
    let mut future = wire[..12].to_vec();
    future[10] = 2;
    assert!(matches!(
        decode_body(&future, Lane::Control, PeerLimits::default()),
        Err(nf_transport::PeerError::Unsupported)
    ));
    assert!(matches!(
        decode_body(&vec![0; 4097], Lane::Control, PeerLimits::default()),
        Err(nf_transport::PeerError::Limit)
    ));
}
