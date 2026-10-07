mod miniature_support;
use nf_contract::identity::*;
use nf_kernel::{AuthorityContext, miniature::*};
#[test]
fn advancing_and_cancelled_batches_match_independent_exact_versioned_records() {
    let world = miniature_support::world();
    let authority = AuthorityContext {
        term: AuthorityTerm(7),
        session: RuntimeSession(9),
    };
    let empty = admit_miniature(&world, vec![], authority, 2).unwrap();
    let batch = settle_miniature(&world, &empty, authority).unwrap();
    let expected = miniature_support::hex(include_str!("fixtures/miniature/batch-empty.hex"));
    assert_eq!(
        encode_miniature_batch(&batch).expect("supported advancing record"),
        expected
    );
    let frontier = admit_miniature(
        &world,
        vec![miniature_support::intent(&world)],
        authority,
        2,
    )
    .unwrap();
    let cancelled = cancel_miniature(
        &world,
        &frontier,
        authority,
        2,
        MiniatureRejection::Cancelled,
    )
    .unwrap();
    let expected = miniature_support::hex(include_str!("fixtures/miniature/batch-cancel.hex"));
    assert_eq!(encode_miniature_batch(&cancelled).unwrap(), expected);
    assert!(nf_kernel::decode_batch(&expected).is_err());
}
#[test]
fn batch_decoder_requires_trusted_before_and_exact_replay_correspondence() {
    let world = miniature_support::world();
    for text in [
        include_str!("fixtures/miniature/batch-empty.hex"),
        include_str!("fixtures/miniature/batch-cancel.hex"),
    ] {
        let bytes = miniature_support::hex(text);
        let decoded =
            decode_miniature_batch(&bytes, &world).expect("supported trusted-source decoder");
        assert_eq!(encode_miniature_batch(&decoded).unwrap(), bytes);
        let next = replay_miniature(&world, &decoded).unwrap();
        assert!(decode_miniature_batch(&bytes, &next).is_err());
        for index in [17, 49, 81, 82, 90] {
            let mut changed = bytes.clone();
            changed[index] ^= 1;
            assert!(decode_miniature_batch(&changed, &world).is_err());
        }
    }
}
