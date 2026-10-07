mod miniature_support;
use nf_kernel::miniature::*;
fn hex(value: &str) -> Vec<u8> {
    value
        .trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn typed_snapshot_matches_independent_full_record_golden() {
    let world = miniature_support::world();
    let expected = hex(include_str!("fixtures/miniature/snapshot-genesis.hex"));
    assert_eq!(expected.len(), 1211);
    assert_eq!(
        encode_miniature_snapshot(&world).expect("supported miniature snapshot"),
        expected
    );
    assert_eq!(
        nf_kernel::decode_snapshot(&expected),
        Err(nf_kernel::Rejection::InvalidValue)
    );
}
#[test]
fn strict_snapshot_decoder_requires_matching_outer_context_and_full_canonical_bytes() {
    let expected = hex(include_str!("fixtures/miniature/snapshot-genesis.hex"));
    assert_eq!(
        decode_miniature_snapshot(&expected).expect("supported decoder"),
        miniature_support::world()
    );
    for changed in [17, 33, 49, 81, 113, 177] {
        let mut bytes = expected.clone();
        bytes[changed] ^= 1;
        assert!(decode_miniature_snapshot(&bytes).is_err());
    }
    let mut trailing = expected.clone();
    trailing.push(0);
    assert!(decode_miniature_snapshot(&trailing).is_err());
    for cut in 0..expected.len() {
        assert!(decode_miniature_snapshot(&expected[..cut]).is_err());
    }
}
#[test]
fn compiled_profile_identity_matches_independent_source_bundle_literal() {
    let manifest = include_str!("fixtures/miniature/source-bundle.json");
    let digest = manifest
        .split("\"bundle_sha256\": \"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert_eq!(
        MINIATURE_IMPLEMENTATION_HASH.as_slice(),
        miniature_support::hex(digest)
    );
}
