use nf_contract::identity::*;
use nf_kernel::*;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
mod support;
#[test]
fn restart_worker_replay() {
    let root = env::temp_dir().join(format!(
        "nf-kernel-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let _cleanup = Cleanup(root.clone());
    let mut reference = None;
    for workers in [1, 2, 4] {
        let directory = root.join(workers.to_string());
        fs::create_dir(&directory).unwrap();
        child(&directory, "begin", workers);
        child(&directory, "resume", workers);
        child(&directory, "replay", workers);
        let resumed = fs::read(directory.join("final.snapshot")).unwrap();
        let replayed = fs::read(directory.join("replayed.snapshot")).unwrap();
        assert_eq!(resumed, replayed);
        let world = decode_snapshot(&resumed).unwrap();
        assert_eq!(
            world.view().relation(support::entity(20)).unwrap().score,
            -443
        );
        assert_eq!(
            world.view().market(support::entity(30)).unwrap().credits,
            170
        );
        assert_eq!(
            world.view().provider(support::provider(40)).unwrap().draws,
            10
        );
        assert_eq!(
            world
                .view()
                .provider(support::provider(40))
                .unwrap()
                .cooldown_until,
            WorldTick(10)
        );
        assert_eq!(world.outcomes().len(), 20);
        if let Some(expected) = &reference {
            assert_eq!(&resumed, expected);
        } else {
            reference = Some(resumed);
        }
        println!(
            "workers={workers} restarted+replayed hash={:02x?}",
            state_hash(&world).unwrap()
        );
    }
}
fn child(directory: &Path, role: &str, workers: usize) {
    let output = fs::File::create(directory.join(format!("{role}.stdout"))).unwrap();
    let errors = fs::File::create(directory.join(format!("{role}.stderr"))).unwrap();
    let mut child = ProcessCommand::new(env::current_exe().unwrap())
        .args(["--exact", "scenario_child", "--nocapture"])
        .env("NF_KERNEL_TEST_DIRECTORY", directory)
        .env("NF_KERNEL_TEST_ROLE", role)
        .env("NF_KERNEL_TEST_WORKERS", workers.to_string())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(errors))
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "role {role}: {}",
                fs::read_to_string(directory.join(format!("{role}.stderr"))).unwrap()
            );
            break;
        }
        if start.elapsed() > Duration::from_secs(15) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("owned foreground child timed out");
        }
        thread::sleep(Duration::from_millis(10));
    }
}
struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn scenario_child() {
    let Ok(directory) = env::var("NF_KERNEL_TEST_DIRECTORY") else {
        return;
    };
    let directory = PathBuf::from(directory);
    let role = env::var("NF_KERNEL_TEST_ROLE").unwrap();
    let workers: usize = env::var("NF_KERNEL_TEST_WORKERS").unwrap().parse().unwrap();
    if role == "replay" {
        let mut world =
            decode_snapshot(&fs::read(directory.join("initial.snapshot")).unwrap()).unwrap();
        for tick in 0..10 {
            let batch =
                decode_batch(&fs::read(directory.join(format!("batch-{tick}"))).unwrap()).unwrap();
            world = apply_batch(&world, &batch).unwrap();
        }
        fs::write(
            directory.join("replayed.snapshot"),
            encode_snapshot(&world).unwrap(),
        )
        .unwrap();
        return;
    }
    let mut world = if role == "begin" {
        support::fixture()
    } else {
        decode_snapshot(&fs::read(directory.join("checkpoint.snapshot")).unwrap()).unwrap()
    };
    if role == "begin" {
        fs::write(
            directory.join("initial.snapshot"),
            encode_snapshot(&world).unwrap(),
        )
        .unwrap();
    }
    let start = if role == "begin" { 0 } else { 5 };
    let end = if role == "begin" { 5 } else { 10 };
    for tick in start..end {
        let authority = AuthorityContext {
            term: AuthorityTerm(workers as u64 + tick as u64 + 1),
            session: RuntimeSession(workers as u64 + tick as u64 + 20),
        };
        let intents = if tick == 5 {
            let persisted =
                decode_frontier(&fs::read(directory.join("admitted.frontier")).unwrap()).unwrap();
            persisted
                .jobs()
                .iter()
                .map(|job| job.intent().clone())
                .collect()
        } else {
            vec![
                support::peace_intent(&world, tick * 2 + 1),
                support::market_intent(&world, tick * 2 + 2, 7),
            ]
        };
        let frontier = admit(&world, intents, authority).unwrap();
        let mut results = parallel(&frontier, workers);
        if workers.is_multiple_of(2) {
            results.reverse();
        }
        let Settlement::Committed { world: next, batch } =
            settle(&world, &frontier, results, authority, MissingPolicy::Pause).unwrap()
        else {
            panic!("complete static jobs")
        };
        fs::write(
            directory.join(format!("batch-{tick}")),
            encode_batch(&batch).unwrap(),
        )
        .unwrap();
        world = *next;
    }
    if role == "begin" {
        fs::write(
            directory.join("checkpoint.snapshot"),
            encode_snapshot(&world).unwrap(),
        )
        .unwrap();
        let frontier = admit(
            &world,
            vec![
                support::peace_intent(&world, 11),
                support::market_intent(&world, 12, 7),
            ],
            AuthorityContext {
                term: AuthorityTerm(1),
                session: RuntimeSession(1),
            },
        )
        .unwrap();
        fs::write(
            directory.join("admitted.frontier"),
            encode_frontier(&frontier).unwrap(),
        )
        .unwrap();
    } else {
        fs::write(
            directory.join("final.snapshot"),
            encode_snapshot(&world).unwrap(),
        )
        .unwrap();
    }
}
fn parallel(frontier: &Frontier, workers: usize) -> Vec<JobResult> {
    let ids: Vec<_> = frontier.jobs().iter().map(Job::id).collect();
    thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|worker| {
                let ids = &ids;
                scope.spawn(move || {
                    ids.iter()
                        .enumerate()
                        .filter(|(index, _)| index % workers == worker)
                        .map(|(_, id)| JobResult {
                            job: *id,
                            result: evaluate(frontier, *id),
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    })
}
