mod support;
use nf_nex_shadow::*;
const WAR: &str = include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv");
const PEACE: &str = include_str!("../../../java/nex-reference/src/test/resources/peace-v1.tsv");
#[test]
fn all_named_independent_source_corpus_cases_match_exact_bits_and_calls() {
    let mut count = 0;
    for row in WAR.lines().filter(|s| !s.starts_with('#') && !s.is_empty()) {
        let f: Vec<_> = row.split('|').collect();
        let input = support::war(&f);
        let result = evaluate_war(&input, WarOperation::Generate).unwrap();
        assert_eq!(
            (
                result.generated,
                result.ended,
                result.abort_current_action,
                result.valid
            ),
            (
                f[8].parse().unwrap(),
                f[9].parse().unwrap(),
                f[10].parse().unwrap(),
                f[11].parse().unwrap()
            ),
            "{}",
            f[0]
        );
        assert_eq!(support::modifiers(&result.writes), f[12], "{}", f[0]);
        assert_eq!(result.existing_priority, input.priority.existing);
        count += 1;
    }
    assert_eq!(count, 10);
    let mut count = 0;
    for row in PEACE
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let f: Vec<_> = row.split('|').collect();
        let result = evaluate_selected_peace(&support::peace(&f)).unwrap();
        assert_eq!(
            result.decision,
            match f[5] {
                "NONE" => PeaceDecision::None,
                "CEASEFIRE" => PeaceDecision::Ceasefire,
                "TREATY" => PeaceDecision::Treaty,
                _ => panic!("bad fixture"),
            }
        );
        assert_eq!(result.consumed_draws, f[6].parse::<u8>().unwrap());
        let expected = if f[7] == "-" {
            "-".into()
        } else {
            format!(
                "event:hegemony:tritachyon:{},weariness:hegemony:{},weariness:tritachyon:{}",
                if f[5] == "TREATY" {
                    "peace_treaty"
                } else {
                    "ceasefire"
                },
                f[7],
                f[7]
            )
        };
        assert_eq!(support::effects(&result.effects), expected, "{}", f[0]);
        count += 1;
    }
    assert_eq!(count, 16);
}
