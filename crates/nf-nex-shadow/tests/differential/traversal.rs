use nf_nex_shadow::{
    PeaceTraversalResult, ReportedPeaceReturn, TraversalEnemy, TraversalStep, Unavailable,
    evaluate_passed_outer_gates,
};

#[test]
#[ignore = "requires NF_SHADOW_JAVA and NF_SHADOW_CLASSPATH_FILE; exportDifferentialClasspath first"]
fn real_java_copied_traversal_matches_rust_from_raw_inputs() {
    let rows: Vec<_> =
        include_str!("../../../../java/nex-reference/src/test/resources/peace-traversal-v1.tsv")
            .lines()
            .filter(|r| !r.starts_with('#') && !r.is_empty())
            .collect();
    assert_eq!(rows.len(), 10);
    let mut rust_output = String::new();
    for row in rows {
        let f: Vec<_> = row.split('|').collect();
        assert_eq!(f.len(), 2); // There is no expected-results column.
        let facts = if f[1] == "-" {
            Vec::new()
        } else {
            f[1].split(';')
                .map(|entry| {
                    let v: Vec<_> = entry.split(',').collect();
                    assert_eq!(v.len(), 7);
                    TraversalEnemy {
                        id: v[0].into(),
                        raw_weariness_bits: u32::from_str_radix(v[1], 16).unwrap(),
                        recent_war: v[2].parse().unwrap(),
                        can_ceasefire: v[3].parse().unwrap(),
                        commissioned_player: v[4].parse().unwrap(),
                        offensive_blocked: v[5].parse().unwrap(),
                        reported_return: match v[6] {
                            "N" => ReportedPeaceReturn::Null,
                            "R" => ReportedPeaceReturn::NonNull,
                            "U" => ReportedPeaceReturn::NotSupplied,
                            _ => panic!("invalid synthetic return"),
                        },
                    }
                })
                .collect()
        };
        let original = facts.clone();
        let result = match evaluate_passed_outer_gates(&facts) {
            Ok(result) => output(&result),
            Err(Unavailable::MissingFact) => "UNAVAILABLE".into(),
            Err(other) => panic!("unexpected synthetic refusal: {other:?}"),
        };
        assert_eq!(facts, original);
        rust_output.push_str(&format!("traversal|{}|{}\n", f[0], result));
    }
    assert_eq!(
        super::oracle("traversal", "peace-traversal-v1.tsv"),
        rust_output
    );
}
fn output(result: &PeaceTraversalResult) -> String {
    let visits = result
        .visits
        .iter()
        .map(|v| {
            format!(
                "{}:{}",
                v.enemy,
                match v.step {
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
