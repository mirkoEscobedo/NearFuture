use nf_nex_shadow::{
    PeaceTraversalResult, ReportedPeaceReturn, TraversalEnemy, TraversalStep, TraversalVisit,
    Unavailable, evaluate_passed_outer_gates,
};

// Source-derived copied-fact traversal characterization; literal public expectations.
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
fn skipped(id: &str, bits: u32) -> TraversalEnemy {
    TraversalEnemy {
        recent_war: true,
        ..enemy(id, bits, ReportedPeaceReturn::NotSupplied)
    }
}
fn visit(id: &str, step: TraversalStep) -> TraversalVisit {
    TraversalVisit {
        enemy: id.into(),
        step,
    }
}

#[test]
fn java_float_edges_have_literal_total_order_and_stable_nan_ties() {
    let supplied = [
        ("negative-nan-first", 0xffc0_0002),
        ("positive-zero", 0x0000_0000),
        ("positive-infinity", 0x7f80_0000),
        ("negative-zero", 0x8000_0000),
        ("negative-infinity", 0xff80_0000),
        ("positive-finite", 0x3f80_0000),
        ("positive-nan-second", 0x7fc0_0001),
        ("negative-finite", 0xbf80_0000),
        ("negative-subnormal", 0x8000_0001),
        ("positive-subnormal", 0x0000_0001),
        ("positive-signaling-third", 0x7f80_0001),
        ("negative-signaling-fourth", 0xff80_0001),
    ]
    .map(|(id, bits)| skipped(id, bits));
    let before = supplied.clone();
    let actual = evaluate_passed_outer_gates(&supplied).unwrap();
    assert_eq!(
        actual.ordered_enemies,
        [
            "negative-infinity",
            "negative-finite",
            "negative-subnormal",
            "negative-zero",
            "positive-zero",
            "positive-subnormal",
            "positive-finite",
            "positive-infinity",
            "negative-nan-first",
            "positive-nan-second",
            "positive-signaling-third",
            "negative-signaling-fourth",
        ]
    );
    assert_eq!(
        supplied, before,
        "raw NaN sign, payload and signaling bits are copied facts"
    );
}

#[test]
fn equal_finite_rankings_keep_input_ties_and_duplicate_occurrences() {
    let supplied = [
        skipped("zeta", 0x3f80_0000),
        skipped("alpha", 0x3f80_0000),
        skipped("zeta", 0x3f80_0000),
    ];
    assert_eq!(
        evaluate_passed_outer_gates(&supplied)
            .unwrap()
            .ordered_enemies,
        ["zeta", "alpha", "zeta"]
    );
}

#[test]
fn four_skips_have_source_precedence_and_do_not_consume_the_three_null_calls() {
    let supplied = [
        TraversalEnemy {
            can_ceasefire: false,
            commissioned_player: true,
            offensive_blocked: true,
            ..skipped("recent", 0x0000_0000)
        },
        TraversalEnemy {
            can_ceasefire: false,
            commissioned_player: true,
            offensive_blocked: true,
            ..enemy("cannot", 0x3f80_0000, ReportedPeaceReturn::NotSupplied)
        },
        TraversalEnemy {
            commissioned_player: true,
            offensive_blocked: true,
            ..enemy("commission", 0x4000_0000, ReportedPeaceReturn::NotSupplied)
        },
        TraversalEnemy {
            offensive_blocked: true,
            ..enemy("offensive", 0x4040_0000, ReportedPeaceReturn::NotSupplied)
        },
        enemy("one", 0x4080_0000, ReportedPeaceReturn::Null),
        enemy("two", 0x40a0_0000, ReportedPeaceReturn::Null),
        enemy("three", 0x40c0_0000, ReportedPeaceReturn::Null),
        enemy("unreached", 0x40e0_0000, ReportedPeaceReturn::NotSupplied),
    ];
    let actual = evaluate_passed_outer_gates(&supplied).unwrap();
    assert_eq!(
        actual.ordered_enemies,
        [
            "recent",
            "cannot",
            "commission",
            "offensive",
            "one",
            "two",
            "three",
            "unreached"
        ]
    );
    assert_eq!(
        actual.visits,
        [
            visit("recent", TraversalStep::RecentWar),
            visit("cannot", TraversalStep::CannotCeasefire),
            visit("commission", TraversalStep::CommissionedPlayer),
            visit("offensive", TraversalStep::OffensiveBlocked),
            visit("one", TraversalStep::NullReturn),
            visit("two", TraversalStep::NullReturn),
            visit("three", TraversalStep::NullReturn)
        ]
    );
    assert_eq!(actual.null_attempts, 3);
    assert_eq!(actual.returned_enemy, None);
    assert_eq!(actual.cache_refresh_bits, Vec::<u32>::new());
}

#[test]
fn first_reported_non_null_return_stops_and_requests_one_positive_zero_refresh() {
    let supplied = [
        enemy("null", 0, ReportedPeaceReturn::Null),
        enemy("returned", 0x3f80_0000, ReportedPeaceReturn::NonNull),
        enemy("unreached", 0x4000_0000, ReportedPeaceReturn::NotSupplied),
    ];
    let actual = evaluate_passed_outer_gates(&supplied).unwrap();
    assert_eq!(actual.ordered_enemies, ["null", "returned", "unreached"]);
    assert_eq!(
        actual.visits,
        [
            visit("null", TraversalStep::NullReturn),
            visit("returned", TraversalStep::NonNullReturn)
        ]
    );
    assert_eq!(actual.null_attempts, 1);
    assert_eq!(actual.returned_enemy.as_deref(), Some("returned"));
    assert_eq!(
        actual.cache_refresh_bits,
        [0x0000_0000],
        "request only; no getter, manager or effect executes"
    );
}

#[test]
fn three_actual_null_calls_stop_before_fourth_without_a_refresh_request() {
    let supplied = [
        enemy("one", 0, ReportedPeaceReturn::Null),
        enemy("two", 0x3f80_0000, ReportedPeaceReturn::Null),
        enemy("three", 0x4000_0000, ReportedPeaceReturn::Null),
        enemy("fourth", 0x4040_0000, ReportedPeaceReturn::NotSupplied),
    ];
    let actual = evaluate_passed_outer_gates(&supplied).unwrap();
    assert_eq!(
        actual.visits,
        [
            visit("one", TraversalStep::NullReturn),
            visit("two", TraversalStep::NullReturn),
            visit("three", TraversalStep::NullReturn)
        ]
    );
    assert_eq!(actual.null_attempts, 3);
    assert_eq!(actual.returned_enemy, None);
    assert_eq!(actual.cache_refresh_bits, Vec::<u32>::new());
}

#[test]
fn reached_eligible_enemy_requires_its_reported_return() {
    assert_eq!(
        evaluate_passed_outer_gates(&[enemy("missing", 0, ReportedPeaceReturn::NotSupplied)]),
        Err(Unavailable::MissingFact)
    );
}

#[test]
fn empty_supplied_list_has_literal_empty_result() {
    assert_eq!(
        evaluate_passed_outer_gates(&[]),
        Ok(PeaceTraversalResult {
            ordered_enemies: Vec::new(),
            visits: Vec::new(),
            null_attempts: 0,
            returned_enemy: None,
            cache_refresh_bits: Vec::new(),
        })
    );
}

#[test]
fn copied_fact_count_accepts_256_and_refuses_257_occurrences() {
    let mut supplied = vec![skipped("entry", 0); 256];
    let actual = evaluate_passed_outer_gates(&supplied).unwrap();
    assert_eq!(actual.ordered_enemies, vec![String::from("entry"); 256]);
    assert_eq!(
        actual.visits,
        vec![visit("entry", TraversalStep::RecentWar); 256]
    );
    assert_eq!(actual.null_attempts, 0);
    supplied.push(skipped("entry", 0));
    assert_eq!(
        evaluate_passed_outer_gates(&supplied),
        Err(Unavailable::Limit)
    );
}

#[test]
fn ids_are_nonempty_and_at_most_128_utf16_units_not_unicode_scalars() {
    let ascii128 = "a".repeat(128);
    let emoji128 = "😀".repeat(64);
    for id in [&ascii128, &emoji128] {
        assert_eq!(
            evaluate_passed_outer_gates(&[skipped(id, 0)])
                .unwrap()
                .ordered_enemies,
            [id.as_str()]
        );
    }
    assert_eq!(
        evaluate_passed_outer_gates(&[skipped("", 0)]),
        Err(Unavailable::MissingFact)
    );
    assert_eq!(
        evaluate_passed_outer_gates(&[skipped(&"a".repeat(129), 0)]),
        Err(Unavailable::Limit)
    );
    assert_eq!(
        evaluate_passed_outer_gates(&[skipped(&(emoji128 + "a"), 0)]),
        Err(Unavailable::Limit)
    );
}
