mod world_support;
use nf_nex_boundary::NexWorld;
use nf_nex_shadow::*;
#[test]
fn full_world_diagnostics_are_identical_across_worker_counts_and_completion_order() {
    let jobs: Vec<_> = (0..8)
        .map(|i| {
            let mut s = world_support::rich_snapshot();
            s.timers.meeting += i;
            let m = world_support::metadata(&s);
            (NexWorld::admit(s).unwrap(), m)
        })
        .collect();
    let run = |(world, m): &(NexWorld, ShadowMetadata)| {
        let mut session = WorldShadowSession::new(m.clone(), world).unwrap();
        let result = session
            .evaluate_war(world, WarOperation::Update, m.concern_instance)
            .unwrap();
        assert!(
            session
                .accept_war(&result, m, world, WarOperation::Update, m.concern_instance)
                .is_ok()
        );
        result
    };
    let expected: Vec<_> = jobs.iter().map(run).collect();
    for workers in [1, 2, 4] {
        for _ in 0..4 {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::scope(|scope| {
                for worker in 0..workers {
                    let tx = tx.clone();
                    let jobs = &jobs;
                    scope.spawn(move || {
                        for index in (0..jobs.len()).rev().filter(|i| i % workers == worker) {
                            tx.send((index, run(&jobs[index]))).unwrap();
                        }
                    });
                }
            });
            drop(tx);
            let mut completed: Vec<_> = rx.into_iter().collect();
            completed.sort_by_key(|(i, _)| *i);
            assert_eq!(
                completed.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
                expected,
                "worker count {workers}"
            );
        }
    }
}
