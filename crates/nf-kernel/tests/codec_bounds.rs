use nf_contract::identity::*;
use nf_kernel::*;
mod support;
#[test]
fn duplicate_durable_outcomes_are_rejected_on_snapshot_restore() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![support::peace_intent(&world, 1)], authority).unwrap();
    let Settlement::Committed { world: next, .. } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("complete")
    };
    let mut bytes = encode_snapshot(&next).unwrap();
    let tail = bytes[bytes.len() - 36..].to_vec();
    let count = bytes.len() - 40;
    bytes[count..count + 4].copy_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&tail);
    assert_eq!(decode_snapshot(&bytes), Err(Rejection::DuplicateIdentity));
}
#[test]
fn malformed_counts_truncation_unsorted_keys_and_unknown_domain_fail_closed() {
    let world = support::fixture();
    let bytes = encode_snapshot(&world).unwrap();
    for cut in 0..bytes.len() {
        assert!(decode_snapshot(&bytes[..cut]).is_err(), "truncation {cut}");
    }
    let mut excessive = bytes.clone();
    excessive[129..133].copy_from_slice(&129u32.to_le_bytes());
    assert_eq!(decode_snapshot(&excessive), Err(Rejection::Limit));
    let mut unsorted = bytes.clone();
    let first = unsorted[133..165].to_vec();
    let second = unsorted[165..197].to_vec();
    unsorted[133..165].copy_from_slice(&second);
    unsorted[165..197].copy_from_slice(&first);
    assert_eq!(decode_snapshot(&unsorted), Err(Rejection::InvalidValue));
    let mut unknown = bytes.clone();
    unknown[11] = 8;
    assert_eq!(decode_snapshot(&unknown), Err(Rejection::InvalidValue));
    assert!(nf_contract::canonical::records::decode_record(&bytes).is_err());
    assert_eq!(decode_snapshot(&vec![0; 1_048_577]), Err(Rejection::Limit));
}
#[test]
fn changed_frontier_input_and_batch_state_hashes_reject() {
    let world = support::fixture();
    let authority = AuthorityContext {
        term: AuthorityTerm(1),
        session: RuntimeSession(1),
    };
    let frontier = admit(&world, vec![support::peace_intent(&world, 1)], authority).unwrap();
    let mut bytes = encode_frontier(&frontier).unwrap();
    let nested = u32::from_le_bytes(bytes[17..21].try_into().unwrap()) as usize;
    bytes[21 + nested] ^= 1;
    assert_eq!(decode_frontier(&bytes), Err(Rejection::InvalidFrontier));
    let Settlement::Committed { mut batch, .. } = settle(
        &world,
        &frontier,
        vec![],
        authority,
        MissingPolicy::Recompute,
    )
    .unwrap() else {
        panic!("complete")
    };
    batch.after_hash[0] ^= 1;
    assert_eq!(apply_batch(&world, &batch), Err(Rejection::StateMismatch));
    assert_eq!(
        world.view().relation(support::entity(20)).unwrap().score,
        -500
    );
}
