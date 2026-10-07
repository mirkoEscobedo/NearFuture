use nf_wire::{Limits, WireError, decode_chunk, decode_chunk_with_limits};
fn raw_chunk() -> Vec<u8> {
    // Independent protobuf tags/widths, never the generated encoder.
    let mut bytes = vec![0x0a, 18, 0x0a, 16];
    bytes.extend_from_slice(&[1; 16]);
    bytes.extend_from_slice(&[0x18, 1, 0x21]);
    bytes.extend_from_slice(&65u64.to_le_bytes());
    bytes.extend_from_slice(&[0x2a, 34, 0x0a, 32]);
    bytes.extend_from_slice(&[2; 32]);
    bytes.extend_from_slice(&[0x32, 65]);
    bytes.extend_from_slice(&[3; 65]);
    bytes
}
#[test]
fn negotiated_chunk_limits_reject_before_generated_payload_allocation() {
    let bytes = raw_chunk();
    let base = Limits::default();
    assert_eq!(decode_chunk(&bytes), decode_chunk_with_limits(&bytes, base));
    assert!(decode_chunk(&bytes).is_ok());
    for limits in [
        Limits {
            frame_bytes: bytes.len() - 1,
            ..base
        },
        Limits {
            field_bytes: 64,
            ..base
        },
        Limits {
            decoded_bytes: 32,
            ..base
        },
        Limits { depth: 1, ..base },
        Limits {
            field_bytes: base.field_bytes + 1,
            ..base
        },
    ] {
        assert_eq!(
            decode_chunk_with_limits(&bytes, limits),
            Err(WireError::Limit)
        );
    }
    let truncated = &bytes[..bytes.len() - 65];
    assert_eq!(decode_chunk(truncated), Err(WireError::Malformed));
    assert_eq!(
        decode_chunk_with_limits(
            truncated,
            Limits {
                field_bytes: 64,
                ..base
            }
        ),
        Err(WireError::Limit)
    );
}
