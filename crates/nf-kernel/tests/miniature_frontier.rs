mod miniature_support;
use nf_contract::identity::*;
use nf_kernel::{AuthorityContext, miniature::*};
#[test]
fn frontier_derives_one_exact_hold_and_invalid_earlier_job_does_not_suppress_legal_winner() {
    let world = miniature_support::world();
    let valid = miniature_support::intent(&world);
    let mut invalid = valid.clone();
    invalid.request = RequestId::from_bytes([27; 16]);
    invalid.operation = OperationId::from_bytes([29; 16]);
    invalid.job = JobId::from_bytes([28; 16]);
    invalid.actor = AccountId::from_bytes([99; 16]);
    for order in [
        vec![valid.clone(), invalid.clone()],
        vec![invalid.clone(), valid.clone()],
    ] {
        let frontier = admit_miniature(
            &world,
            order,
            AuthorityContext {
                term: AuthorityTerm(7),
                session: RuntimeSession(9),
            },
            2,
        )
        .expect("supported miniature frontier");
        let hold = frontier.reservation().unwrap();
        assert_eq!((hold.credits(), hold.supplies()), (100, 40));
        assert_eq!(hold.operation(), valid.operation);
        assert_eq!(
            frontier.outcomes()[0].rejection,
            Some(MiniatureRejection::Unauthorized)
        );
        assert_eq!(frontier.outcomes()[1].rejection, None);
        assert_eq!(frontier.committing_tick(), WorldTick(1));
    }
}
#[test]
fn typed_frontier_commits_exact_source_and_only_derived_hold_in_independent_golden() {
    let world = miniature_support::world();
    let intent = miniature_support::intent(&world);
    let frontier = admit_miniature(
        &world,
        vec![intent],
        AuthorityContext {
            term: AuthorityTerm(7),
            session: RuntimeSession(9),
        },
        2,
    )
    .unwrap();
    let expected = miniature_support::hex(include_str!("fixtures/miniature/frontier-colony.hex"));
    assert_eq!(
        encode_miniature_frontier(&frontier).expect("supported frontier codec"),
        expected
    );
    assert!(nf_kernel::decode_frontier(&expected).is_err());
}
#[test]
fn frontier_decoder_recomputes_hold_and_fences_exact_next_tick_before_returning_private_plan() {
    let bytes = miniature_support::hex(include_str!("fixtures/miniature/frontier-colony.hex"));
    let restored = decode_miniature_frontier(&bytes).expect("supported bounded frontier decoder");
    assert_eq!(encode_miniature_frontier(&restored).unwrap(), bytes);
    let mut hold = bytes.clone();
    let credit = hold.len() - 16;
    hold[credit] = 99;
    assert!(decode_miniature_frontier(&hold).is_err());
    let mut delayed = bytes.clone();
    delayed[65..73].copy_from_slice(&2u64.to_le_bytes());
    assert!(decode_miniature_frontier(&delayed).is_err());
    let mut changed_source = bytes.clone();
    changed_source[17] ^= 1;
    assert!(decode_miniature_frontier(&changed_source).is_err());
    let world = miniature_support::world();
    let intent = miniature_support::intent(&world);
    let fresh = admit_miniature(&world, vec![intent.clone()], restored.authority(), 3).unwrap();
    assert_eq!(restored.intents(), fresh.intents());
    assert_eq!(
        miniature_request_binding(&intent).unwrap().payload_digest,
        miniature_request_binding(&fresh.intents()[0])
            .unwrap()
            .payload_digest
    );
    assert_ne!(
        encode_miniature_frontier(&restored).unwrap(),
        encode_miniature_frontier(&fresh).unwrap()
    );
}
