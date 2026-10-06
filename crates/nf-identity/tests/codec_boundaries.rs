mod support;
use nf_identity::{codec::*, model::IdentityError};
#[test]
fn identity_extension_remains_closed_bounded_and_distinct_from_wire_baseline() {
    let c = support::Community::new();
    let invite = c.invitation();
    let bytes = encode_invitation(&invite.invitation).unwrap();
    assert_eq!(&bytes[..17], b"NF-CANON-1\0\x08\0\x01\0\x01\0");
    assert_eq!(decode_invitation(&bytes).unwrap(), invite.invitation);
    assert!(nf_contract::canonical::records::decode_record(&bytes).is_err());
    let state = encode_state(&c.state).unwrap();
    assert_eq!(decode_state(&state).unwrap(), c.state);
    let mut bad_count = state.clone();
    bad_count[73..77].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(decode_state(&bad_count), Err(IdentityError::Limit));
    assert_eq!(
        decode_state(&vec![0; MAX_STATE_BYTES + 1]),
        Err(IdentityError::Limit)
    );
    let mut seed = 0x91abd1u64;
    for i in 0..10_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut candidate = if i % 2 == 0 {
            bytes.clone()
        } else {
            state.clone()
        };
        let index = (seed as usize) % candidate.len();
        candidate[index] ^= (seed >> 32) as u8;
        if seed & 3 == 0 {
            candidate.truncate(index);
        }
        if i % 2 == 0 {
            if let Ok(parsed) = decode_invitation(&candidate) {
                assert_eq!(encode_invitation(&parsed).unwrap(), candidate);
            }
        } else if let Ok(parsed) = decode_state(&candidate) {
            assert_eq!(encode_state(&parsed).unwrap(), candidate);
        }
    }
}
