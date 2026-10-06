mod support;
use nf_nex_boundary::{BoundaryError, NexWorld};

#[test]
fn unrecognized_merged_priority_tag_is_not_silently_ignored() {
    let mut snapshot = support::snapshot();
    snapshot
        .concern_config
        .tags
        .push("third_party_semantics".into());
    assert_eq!(
        NexWorld::admit(snapshot),
        Err(BoundaryError::InvalidFact("concern tags"))
    );
}
