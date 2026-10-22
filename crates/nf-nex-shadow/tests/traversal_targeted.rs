use nf_nex_shadow::{
    PeaceTraversalResult, ReportedPeaceReturn, TraversalEnemy, TraversalStep, TraversalVisit,
    evaluate_selection_passed_outer_gates,
};

// Source-derived target selection after outer guards; every expected trace is literal.
fn enemy(id: &str, bits: u32, returned: ReportedPeaceReturn) -> TraversalEnemy {
    TraversalEnemy {
        id: id.into(),
        raw_weariness_bits: bits,
        recent_war: false,
        can_ceasefire: true,
        commissioned_player: false,
        offensive_blocked: false,
        reported_return: returned,
    }
}
#[test]
fn explicit_target_not_in_pool_is_the_only_attempt_even_with_higher_score() {
    let pool = vec![enemy("low", 0x3f80_0000, ReportedPeaceReturn::NonNull)];
    let target = enemy("target", 0x4110_0000, ReportedPeaceReturn::NonNull);
    let before_pool = pool.clone();
    let before_target = target.clone();
    let actual = evaluate_selection_passed_outer_gates(&pool, Some(&target)).unwrap();
    assert_eq!(
        actual,
        PeaceTraversalResult {
            ordered_enemies: vec!["target".into()],
            visits: vec![TraversalVisit {
                enemy: "target".into(),
                step: TraversalStep::NonNullReturn
            }],
            null_attempts: 0,
            returned_enemy: Some("target".into()),
            cache_refresh_bits: vec![0x0000_0000],
        }
    );
    assert_eq!(pool, before_pool);
    assert_eq!(target, before_target);
}
#[test]
fn targeted_null_does_not_fall_back_to_an_available_pool_enemy() {
    let pool = vec![enemy("low", 0x3f80_0000, ReportedPeaceReturn::NonNull)];
    let target = enemy("target", 0x4110_0000, ReportedPeaceReturn::Null);
    assert_eq!(
        evaluate_selection_passed_outer_gates(&pool, Some(&target)).unwrap(),
        PeaceTraversalResult {
            ordered_enemies: vec!["target".into()],
            visits: vec![TraversalVisit {
                enemy: "target".into(),
                step: TraversalStep::NullReturn
            }],
            null_attempts: 1,
            returned_enemy: None,
            cache_refresh_bits: vec![],
        }
    );
}
#[test]
fn blocked_target_skips_without_return_or_fallback() {
    let pool = vec![enemy("low", 0x3f80_0000, ReportedPeaceReturn::NonNull)];
    let target = TraversalEnemy {
        offensive_blocked: true,
        ..enemy("target", 0x4110_0000, ReportedPeaceReturn::NotSupplied)
    };
    assert_eq!(
        evaluate_selection_passed_outer_gates(&pool, Some(&target)).unwrap(),
        PeaceTraversalResult {
            ordered_enemies: vec!["target".into()],
            visits: vec![TraversalVisit {
                enemy: "target".into(),
                step: TraversalStep::OffensiveBlocked
            }],
            null_attempts: 0,
            returned_enemy: None,
            cache_refresh_bits: vec![],
        }
    );
}
#[test]
fn absent_target_uses_the_existing_untargeted_route() {
    let pool = vec![
        enemy("high", 0x4110_0000, ReportedPeaceReturn::NonNull),
        enemy("low", 0x3f80_0000, ReportedPeaceReturn::Null),
    ];
    assert_eq!(
        evaluate_selection_passed_outer_gates(&pool, None).unwrap(),
        PeaceTraversalResult {
            ordered_enemies: vec!["low".into(), "high".into()],
            visits: vec![
                TraversalVisit {
                    enemy: "low".into(),
                    step: TraversalStep::NullReturn
                },
                TraversalVisit {
                    enemy: "high".into(),
                    step: TraversalStep::NonNullReturn
                }
            ],
            null_attempts: 1,
            returned_enemy: Some("high".into()),
            cache_refresh_bits: vec![0x0000_0000],
        }
    );
}
