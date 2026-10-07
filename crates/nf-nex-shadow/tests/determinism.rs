mod support;
use nf_nex_shadow::*;
use std::thread;
#[test]
fn worker_counts_repeat_runs_and_completion_order_preserve_all_named_outputs() {
    let mut jobs: Vec<ShadowInput> =
        include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
            .lines()
            .filter(|r| !r.starts_with('#') && !r.is_empty())
            .map(|r| ShadowInput::War {
                operation: WarOperation::Generate,
                facts: support::war(&r.split('|').collect::<Vec<_>>()),
            })
            .collect();
    jobs.extend(
        include_str!("../../../java/nex-reference/src/test/resources/peace-v1.tsv")
            .lines()
            .filter(|r| !r.starts_with('#') && !r.is_empty())
            .map(|r| ShadowInput::SelectedPeace(support::peace(&r.split('|').collect::<Vec<_>>()))),
    );
    assert_eq!(jobs.len(), 26);
    let calculate = |job: &ShadowInput| match job {
        ShadowInput::War { operation, facts } => {
            ShadowOutput::War(evaluate_war(facts, *operation).unwrap())
        }
        ShadowInput::SelectedPeace(facts) => {
            ShadowOutput::SelectedPeace(evaluate_selected_peace(facts).unwrap())
        }
    };
    let expected: Vec<_> = jobs.iter().map(calculate).collect();
    for workers in [1, 2, 4] {
        for _ in 0..8 {
            let mut actual = thread::scope(|scope| {
                let mut handles = vec![];
                for worker in 0..workers {
                    let jobs = &jobs;
                    handles.push(scope.spawn(move || {
                        jobs.iter()
                            .enumerate()
                            .rev()
                            .filter(|(i, _)| i % workers == worker)
                            .map(|(i, j)| (i, calculate(j)))
                            .collect::<Vec<_>>()
                    }));
                }
                handles
                    .into_iter()
                    .rev()
                    .flat_map(|h| h.join().unwrap())
                    .collect::<Vec<_>>()
            });
            actual.sort_by_key(|(i, _)| *i);
            assert_eq!(
                actual.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
                expected
            );
        }
    }
}
#[test]
fn unsupported_facts_fail_without_mutating_copied_inputs() {
    let row = include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
        .lines()
        .find(|r| r.starts_with("equal|"))
        .unwrap();
    let mut input = support::war(&row.split('|').collect::<Vec<_>>());
    input.priority.max_alignment = support::float(f32::MAX);
    input.priority.alignment = support::float(f32::MAX);
    let before = input.clone();
    assert_eq!(
        evaluate_war(&input, WarOperation::Generate),
        Err(Unavailable::NonFinite)
    );
    assert_eq!(input, before);
    input.existing = vec![
        ExistingConcern {
            class: ConcernClass::Other,
            ended: false
        };
        257
    ];
    assert_eq!(
        evaluate_war(&input, WarOperation::Generate),
        Err(Unavailable::Limit)
    );
    let row = include_str!("../../../java/nex-reference/src/test/resources/peace-v1.tsv")
        .lines()
        .find(|r| r.starts_with("chance_equal|"))
        .unwrap();
    let mut peace = support::peace(&row.split('|').collect::<Vec<_>>());
    peace.gates.offensive_facts_provided = false;
    let before = peace.clone();
    assert_eq!(
        evaluate_selected_peace(&peace),
        Err(Unavailable::MissingFact)
    );
    assert_eq!(peace, before);
}

#[test]
fn copied_modifier_ids_use_conservative_128_utf8_byte_domain() {
    let row = include_str!("../../../java/nex-reference/src/test/resources/war-v1.tsv")
        .lines()
        .find(|r| r.starts_with("equal|"))
        .unwrap();
    let mut input = support::war(&row.split('|').collect::<Vec<_>>());
    input.priority.existing[0].id = "x".repeat(128);
    assert!(evaluate_war(&input, WarOperation::Generate).is_ok());
    input.priority.existing[0].id.push('x');
    assert_eq!(
        evaluate_war(&input, WarOperation::Generate),
        Err(Unavailable::Limit)
    );
}
