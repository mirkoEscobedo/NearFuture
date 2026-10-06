use nf_kernel::*;
#[path = "support/quota.rs"]
mod quota;
#[test]
fn global_and_combined_counts_reject_before_missing_or_malformed_tuple_reads() {
    let snapshot = quota::large_snapshot();
    assert_eq!(snapshot.len(), 188577);
    let large = decode_snapshot(&snapshot).unwrap();
    assert_eq!(encode_snapshot(&large).unwrap(), snapshot);
    let bounded = quota::bounded_frontier();
    assert_eq!(
        encode_frontier(&decode_frontier(&bounded).unwrap()).unwrap(),
        bounded
    );
    let full = quota::excessive_frontier(false);
    assert_eq!(full.len(), 444970);
    let rejected = [
        decode_snapshot(&quota::excessive_combined_entities()).err(),
        decode_frontier(&quota::excessive_frontier(true)).err(),
        decode_frontier(&full).err(),
    ];
    assert_eq!(rejected, [Some(Rejection::Limit); 3]);
}
