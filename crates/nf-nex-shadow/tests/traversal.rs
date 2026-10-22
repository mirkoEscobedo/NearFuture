use nf_nex_shadow::{ReportedPeaceReturn, TraversalEnemy, evaluate_passed_outer_gates};

/// Literal source-derived public expectation; the expected list is not sorted by test code.
#[test]
fn ascending_positive_rankings_use_the_source_comparator() {
    let facts = [
        ("high", 0x4040_0000),
        ("low", 0x3f80_0000),
        ("fourth", 0x4080_0000),
        ("middle", 0x4000_0000),
    ]
    .map(|(id, raw_weariness_bits)| TraversalEnemy {
        id: id.into(),
        raw_weariness_bits,
        recent_war: true,
        can_ceasefire: true,
        commissioned_player: false,
        offensive_blocked: false,
        reported_return: ReportedPeaceReturn::NotSupplied,
    });
    let actual = evaluate_passed_outer_gates(&facts).unwrap();
    assert_eq!(
        actual.ordered_enemies,
        ["low", "middle", "high", "fourth"],
        "actual comparator is ascending despite the descending source comment"
    );
}
