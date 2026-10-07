mod support;
use nf_nex_shadow::*;
#[test]
fn mismatch_stream_is_bounded_coalesced_and_does_not_disclose_raw_facts() {
    let row = include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
        .lines()
        .find(|r| r.starts_with("equal|"))
        .unwrap();
    let facts = support::war(&row.split('|').collect::<Vec<_>>());
    let a = ShadowOutput::War(evaluate_war(&facts, WarOperation::Generate).unwrap());
    let mut b = a.clone();
    if let ShadowOutput::War(r) = &mut b {
        r.ended = true;
        r.writes.clear();
    }
    assert_eq!(
        compare_outputs(&a, &b),
        vec![Difference::Lifecycle, Difference::Priority]
    );
    let mut stream = DiagnosticStream::new(1, 161).unwrap();
    let entry = Diagnostic {
        source_digest: [1; 32],
        config_digest: [2; 32],
        input_digest: [3; 32],
        corpus_digest: [4; 32],
        implementation_digest: [5; 32],
        difference: Difference::Lifecycle,
    };
    stream.push(entry.clone());
    stream.push(entry.clone());
    assert_eq!(stream.dropped(), 0);
    let mut next = entry.clone();
    next.input_digest[0] ^= 1;
    stream.push(next);
    assert_eq!(stream.dropped(), 1);
    assert_eq!(stream.pop(), Some(entry));
    assert!(stream.pop().is_none());
}
