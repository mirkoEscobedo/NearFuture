mod support;
use nf_contract::identity::*;
use nf_world::*;
#[test]
fn component_rejects_unknown_versions_limits_truncation_order_and_invalid_values() {
    let s = support::state();
    let bytes = encode_component(&s, WorldTick(0)).unwrap();
    for end in 0..bytes.len() {
        assert!(
            decode_component(&bytes[..end], WorldTick(0)).is_err(),
            "truncation {end}"
        );
    }
    let mut b = bytes.clone();
    b.push(0);
    assert_eq!(
        decode_component(&b, WorldTick(0)).err(),
        Some(WorldError::Malformed)
    );
    for offset in [11, 13, 15, 81] {
        let mut b = bytes.clone();
        b[offset] ^= 1;
        assert_eq!(
            decode_component(&b, WorldTick(0)).err(),
            Some(WorldError::Unsupported)
        );
    }
    let mut b = bytes.clone();
    b[169..173].copy_from_slice(&4u32.to_le_bytes());
    assert_eq!(
        decode_component(&b, WorldTick(0)).err(),
        Some(WorldError::Limit)
    );
    let mut b = bytes.clone();
    b[1014..1018].copy_from_slice(&33u32.to_le_bytes());
    assert_eq!(
        decode_component(&b, WorldTick(0)).err(),
        Some(WorldError::Limit)
    );
    assert_eq!(
        decode_component(&vec![0; 131073], WorldTick(0)).err(),
        Some(WorldError::Limit)
    );
    let mut b = bytes.clone();
    b[173..190].copy_from_slice(&bytes[190..207]);
    b[190..207].copy_from_slice(&bytes[173..190]);
    assert_eq!(
        decode_component(&b, WorldTick(0)).err(),
        Some(WorldError::Malformed)
    );
    let mut b = bytes.clone();
    b[894..902].copy_from_slice(&10001i64.to_le_bytes());
    assert!(decode_component(&b, WorldTick(0)).is_err());
    assert!(
        nf_contract::canonical::records::decode_record(&bytes).is_err(),
        "foundation stays closed"
    );
}
#[test]
fn finite_mutation_corpus_never_panics_or_admits_alternative_encodings() {
    let s = support::state();
    let bytes = encode_component(&s, WorldTick(0)).unwrap();
    let mut seed = 0x918245u64;
    for _ in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut b = bytes.clone();
        let i = (seed as usize) % b.len();
        b[i] ^= 1u8 << ((seed >> 32) % 8);
        if let Ok(decoded) = decode_component(&b, WorldTick(0)) {
            assert_eq!(encode_component(&decoded, WorldTick(0)).unwrap(), b);
            validate_at(&decoded, WorldTick(0)).unwrap();
        }
    }
}
