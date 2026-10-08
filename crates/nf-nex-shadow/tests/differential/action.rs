use nf_nex_shadow::{MakePeaceEligibility, make_peace_action_eligible};

/// Independent real JVM comparison from five raw supplied-fact rows, without expected columns.
#[test]
#[ignore = "requires NF_SHADOW_JAVA and NF_SHADOW_CLASSPATH_FILE; exportDifferentialClasspath first"]
fn real_java_action_eligibility_matches_rust_from_five_supplied_fact_cases() {
    let rows: Vec<_> =
        include_str!("../../../../java/nex-reference/src/test/resources/make-peace-action-v1.tsv")
            .lines()
            .filter(|row| !row.starts_with('#') && !row.is_empty())
            .collect();
    assert_eq!(rows.len(), 5);
    let mut actual = String::new();
    for row in rows {
        let fields: Vec<_> = row.split('|').collect();
        assert_eq!(fields.len(), 6);
        let diplomacy_enabled: bool = fields[1].parse().unwrap();
        let concern_can_make_peace: bool = fields[2].parse().unwrap();
        let has_target: bool = fields[3].parse().unwrap();
        let hostile: bool = fields[4].parse().unwrap();
        let faction_diplomacy_disabled: bool = fields[5].parse().unwrap();
        let facts = MakePeaceEligibility {
            diplomacy_enabled,
            concern_can_make_peace,
            target_hostile: has_target.then_some(hostile),
            faction_diplomacy_disabled,
        };
        let original = facts;
        let eligible = make_peace_action_eligible(&facts).unwrap();
        assert_eq!(facts, original);
        actual.push_str(&format!("action|{}|{}\n", fields[0], eligible));
    }
    assert_eq!(super::oracle("action", "make-peace-action-v1.tsv"), actual);
}
