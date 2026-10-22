use nf_nex_shadow::{Diagnostic, DiagnosticStream, Difference};

#[test]
fn all_distinct_first_proposal_facets_remain_visible_without_draining_or_merging() {
    let expected = [
        Difference::Kind,
        Difference::Lifecycle,
        Difference::Priority,
        Difference::Decision,
        Difference::Draws,
        Difference::Effects,
        Difference::FirstConcernKind,
        Difference::FirstConcernLifecycle,
        Difference::FirstConcernPriority,
        Difference::FirstEligibilityKind,
        Difference::FirstEligibilityDecision,
        Difference::FirstActionPriority,
        Difference::FirstProposalKind,
        Difference::FirstProposalDecision,
        Difference::FirstProposalDraws,
        Difference::FirstProposalEffects,
    ];
    let mut stream = DiagnosticStream::new(32, 5152).unwrap();
    for difference in expected {
        let record = Diagnostic {
            source_digest: [1; 32],
            config_digest: [2; 32],
            input_digest: [3; 32],
            corpus_digest: [4; 32],
            implementation_digest: [5; 32],
            difference,
        };
        stream.push(record.clone());
        stream.push(record);
    }
    for _ in 0..2 {
        let observation = stream.redacted_observation();
        assert_eq!(observation.queued_records(), 16);
        assert_eq!(
            observation
                .difference_counts()
                .iter()
                .map(|count| (count.difference(), count.count()))
                .collect::<Vec<_>>(),
            expected.map(|difference| (difference, 1))
        );
    }
    for difference in expected {
        assert_eq!(stream.pop().unwrap().difference, difference);
    }
    assert!(stream.pop().is_none());
    assert_eq!(stream.dropped(), 0);
}
