mod driver_support;
use nf_world_driver::Driver;
use std::process::Command;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|n| format!("{n:02x}")).collect()
}
fn base(f: &driver_support::Fixture, verb: &str) -> Command {
    let o = &f.options;
    let mut c = Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    c.arg(verb)
        .arg("--database")
        .arg(&o.database)
        .arg("--vault")
        .arg(&o.vault)
        .arg("--game-save-root")
        .arg(&o.game_save_root)
        .args([
            "--universe",
            &hex(o.policy.scope.universe.as_bytes()),
            "--history",
            &hex(o.policy.scope.history.as_bytes()),
            "--min-event",
            "0",
            "--min-store",
            "0",
            "--min-membership",
            "2",
            "--min-term",
            "0",
            "--account",
            &hex(o.policy.owner.account.as_bytes()),
            "--device",
            &hex(o.policy.owner.device.as_bytes()),
            "--claim-authority",
            "yes",
        ]);
    c
}
fn step(f: &driver_support::Fixture, active: &str) -> Command {
    let mut c = base(f, "step");
    c.args([
        "--active",
        active,
        "--request",
        &"28".repeat(16),
        "--operation",
        &"29".repeat(16),
        "--job",
        &"2a".repeat(16),
        "--action",
        "colony:1:0",
    ]);
    c
}
fn successful(c: Command) -> String {
    let o = driver_support::process::run(c);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
#[test]
fn actual_step_retry_and_paused_run_leave_authority_and_world_unchanged_until_explicit_active_step()
{
    let f = driver_support::Fixture::new();
    drop(Driver::create(f.options.clone()).unwrap());
    let paused = successful(step(&f, "no"));
    assert!(paused.contains("tick=0\n") && paused.contains("authority_term=0\n"));
    let committed = successful(step(&f, "yes"));
    assert!(committed.contains("tick=1\n") && committed.contains("authority_term=1\n"));
    assert_eq!(
        successful(step(&f, "yes")),
        committed,
        "committed retry does not claim a new session or spend twice"
    );
    let mut inactive = base(&f, "run");
    inactive.args([
        "--active",
        "no",
        "--period-ms",
        "100",
        "--duration-seconds",
        "1",
    ]);
    assert_eq!(successful(inactive), committed);
    let mut active = base(&f, "run");
    active.args([
        "--active",
        "yes",
        "--period-ms",
        "60000",
        "--duration-seconds",
        "1",
    ]);
    let result = successful(active);
    assert!(result.contains("tick=1\n") && result.contains("authority_term=2\n"));
    assert_eq!(
        std::fs::read_dir(&f.options.game_save_root)
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn retained_pending_cli_requires_exact_binding_and_explicit_resume_or_cancel_and_run_pauses() {
    use nf_contract::identity::*;
    use nf_world_driver::{ActionRequest, ActionSelection};
    let f = driver_support::Fixture::new();
    let mut driver = Driver::create(f.options.clone()).unwrap();
    driver.claim_authority().unwrap();
    let action = ActionRequest {
        request: RequestId::from_bytes([40; 16]),
        operation: OperationId::from_bytes([41; 16]),
        job: JobId::from_bytes([42; 16]),
        action: ActionSelection::Colony {
            site: 1,
            faction: 0,
        },
    };
    driver.prepare_action(&action).unwrap();
    let before = driver.status().unwrap();
    drop(driver);
    let paused = successful(step(&f, "yes"));
    assert!(paused.contains("tick=0\n") && paused.contains("pending_jobs=1\n"));
    let mut run = base(&f, "run");
    run.args([
        "--active",
        "yes",
        "--period-ms",
        "100",
        "--duration-seconds",
        "1",
    ]);
    assert_eq!(successful(run), paused);
    let selection = step(&f, "yes");
    let mut changed_args = selection
        .get_args()
        .map(|a| a.to_os_string())
        .collect::<Vec<_>>();
    let index = changed_args.iter().position(|a| a == "--action").unwrap() + 1;
    changed_args[index] = "colony:3:0".into();
    let mut changed = Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    changed.args(changed_args);
    let output = driver_support::process::run(changed);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(successful(step(&f, "yes")), paused);
    let mut resume = step(&f, "yes");
    resume.args(["--pending-policy", "resume"]);
    let done = successful(resume);
    assert!(
        done.contains("tick=1\n")
            && done.contains("authority_term=2\n")
            && done.contains("pending_jobs=0\n")
    );
    assert_eq!(successful(step(&f, "yes")), done);
    assert_eq!(before.world.metadata().tick, WorldTick(0));
    // Separate explicit retained cancellation releases the hold without spending or advancing.
    let mut other = f.options.clone();
    other.database = f.scratch.0.join("cancel.sqlite");
    let mut driver = Driver::create(other.clone()).unwrap();
    driver.claim_authority().unwrap();
    driver.prepare_action(&action).unwrap();
    drop(driver);
    let mut cancel = step(&f, "yes");
    let args = cancel
        .get_args()
        .map(|a| a.to_os_string())
        .collect::<Vec<_>>();
    let mut cancel_args = args;
    let db = cancel_args.iter().position(|a| a == "--database").unwrap() + 1;
    cancel_args[db] = other.database.into_os_string();
    cancel = Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    cancel
        .args(cancel_args)
        .args(["--pending-policy", "cancel"]);
    let cancelled = successful(cancel);
    assert!(
        cancelled.contains("tick=0\n")
            && cancelled.contains("event_sequence=1\n")
            && cancelled.contains("pending_jobs=0\n")
    );
}
