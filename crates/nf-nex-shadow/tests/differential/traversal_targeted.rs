use nf_nex_shadow::{
    PeaceTraversalResult, ReportedPeaceReturn, TraversalEnemy, TraversalStep, Unavailable,
    evaluate_selection_passed_outer_gates,
};

#[test]
#[ignore = "requires NF_SHADOW_JAVA and NF_SHADOW_CLASSPATH_FILE; exportDifferentialClasspath first"]
fn real_java_copied_target_selection_matches_rust_from_raw_inputs() {
    let rows: Vec<_> = include_str!(
        "../../../../java/nex-reference/src/test/resources/peace-traversal-targeted-v1.tsv"
    )
    .lines()
    .filter(|row| !row.starts_with('#') && !row.is_empty())
    .collect();
    assert_eq!(rows.len(), 5);
    let mut rust_output = String::new();
    for row in rows {
        let fields: Vec<_> = row.split('|').collect();
        assert_eq!(fields.len(), 3); // No expected-result column.
        let pool = facts(fields[1]);
        let target = if fields[2] == "-" {
            None
        } else {
            let mut supplied = facts(fields[2]);
            assert_eq!(supplied.len(), 1);
            supplied.pop()
        };
        let original_pool = pool.clone();
        let original_target = target.clone();
        let result = match evaluate_selection_passed_outer_gates(&pool, target.as_ref()) {
            Ok(result) => output(&result),
            Err(Unavailable::MissingFact) => "UNAVAILABLE".into(),
            Err(other) => panic!("unexpected synthetic refusal: {other:?}"),
        };
        assert_eq!(pool, original_pool);
        assert_eq!(target, original_target);
        rust_output.push_str(&format!("traversal_targeted|{}|{}\n", fields[0], result));
    }
    assert_eq!(
        super::oracle("traversal_targeted", "peace-traversal-targeted-v1.tsv"),
        rust_output
    );
}
fn facts(text: &str) -> Vec<TraversalEnemy> {
    if text == "-" {
        return Vec::new();
    }
    text.split(';')
        .map(|entry| {
            let fields: Vec<_> = entry.split(',').collect();
            assert_eq!(fields.len(), 7);
            TraversalEnemy {
                id: fields[0].into(),
                raw_weariness_bits: u32::from_str_radix(fields[1], 16).unwrap(),
                recent_war: fields[2].parse().unwrap(),
                can_ceasefire: fields[3].parse().unwrap(),
                commissioned_player: fields[4].parse().unwrap(),
                offensive_blocked: fields[5].parse().unwrap(),
                reported_return: match fields[6] {
                    "N" => ReportedPeaceReturn::Null,
                    "R" => ReportedPeaceReturn::NonNull,
                    "U" => ReportedPeaceReturn::NotSupplied,
                    _ => panic!("invalid synthetic return"),
                },
            }
        })
        .collect()
}
fn output(result: &PeaceTraversalResult) -> String {
    let visits = result
        .visits
        .iter()
        .map(|visit| {
            format!(
                "{}:{}",
                visit.enemy,
                match visit.step {
                    TraversalStep::RecentWar => "RECENT_WAR",
                    TraversalStep::CannotCeasefire => "CANNOT_CEASEFIRE",
                    TraversalStep::CommissionedPlayer => "COMMISSIONED_PLAYER",
                    TraversalStep::OffensiveBlocked => "OFFENSIVE_BLOCKED",
                    TraversalStep::NullReturn => "NULL_RETURN",
                    TraversalStep::NonNullReturn => "NON_NULL_RETURN",
                }
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let refresh = result
        .cache_refresh_bits
        .iter()
        .map(|bits| format!("{bits:08x}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "OK|{}|{}|{}|{}|{}",
        nonempty(result.ordered_enemies.join(",")),
        nonempty(visits),
        result.null_attempts,
        result.returned_enemy.as_deref().unwrap_or("-"),
        nonempty(refresh)
    )
}
fn nonempty(text: String) -> String {
    if text.is_empty() { "-".into() } else { text }
}
