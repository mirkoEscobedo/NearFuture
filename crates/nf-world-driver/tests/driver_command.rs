use nf_contract::identity::*;
use nf_world_driver::{Command, parse_command};
fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}
#[test]
fn read_only_status_requires_explicit_scope_and_protected_backup_minima() {
    let values = args(&[
        "status",
        "--database",
        "private/world.sqlite",
        "--universe",
        "01010101010101010101010101010101",
        "--history",
        "02020202020202020202020202020202",
        "--min-event",
        "7",
        "--min-store",
        "9",
        "--min-membership",
        "3",
        "--min-term",
        "2",
    ]);
    let Command::Status(options) = parse_command(&values).expect("bounded status command") else {
        panic!("status")
    };
    assert_eq!(options.minimum_event, EventSeq(7));
    assert_eq!(options.minimum_store, 9);
    assert_eq!(options.minimum_membership, 3);
    assert_eq!(options.minimum_term, AuthorityTerm(2));
    assert_eq!(options.scope.universe, UniverseId::from_bytes([1; 16]));
    assert_eq!(options.database.to_str(), Some("private/world.sqlite"));
    assert!(parse_command(&values[..values.len() - 2]).is_err());
}
#[test]
fn foreground_run_is_finite_explicit_activity_and_authority_without_default_key_creation() {
    let mut values = args(&[
        "run",
        "--database",
        "private/world.sqlite",
        "--universe",
        "01010101010101010101010101010101",
        "--history",
        "02020202020202020202020202020202",
        "--min-event",
        "0",
        "--min-store",
        "0",
        "--min-membership",
        "2",
        "--min-term",
        "0",
        "--vault",
        "private/key",
        "--game-save-root",
        "saves",
        "--account",
        "04040404040404040404040404040404",
        "--device",
        "05050505050505050505050505050505",
        "--claim-authority",
        "yes",
        "--active",
        "no",
        "--period-ms",
        "100",
        "--duration-seconds",
        "60",
    ]);
    let Command::Run(run) = parse_command(&values).expect("bounded explicit run") else {
        panic!("run")
    };
    assert!(!run.active);
    assert_eq!((run.period_ms, run.duration_seconds), (100, 60));
    assert_eq!(run.signer.device, DeviceId::from_bytes([5; 16]));
    *values.last_mut().unwrap() = "61".into();
    assert!(parse_command(&values).is_err());
    *values.last_mut().unwrap() = "1".into();
    let claim = values
        .iter()
        .position(|s| s == "--claim-authority")
        .unwrap()
        + 1;
    values[claim] = "no".into();
    assert!(parse_command(&values).is_err());
    assert!(parse_command(&args(&["run"])).is_err());
}
#[test]
fn step_keeps_explicit_request_operation_job_and_closed_action_independent_of_fresh_admission() {
    let mut values = args(&[
        "step",
        "--database",
        "private/world.sqlite",
        "--universe",
        "01010101010101010101010101010101",
        "--history",
        "02020202020202020202020202020202",
        "--min-event",
        "0",
        "--min-store",
        "0",
        "--min-membership",
        "2",
        "--min-term",
        "0",
        "--vault",
        "private/key",
        "--game-save-root",
        "saves",
        "--account",
        "04040404040404040404040404040404",
        "--device",
        "05050505050505050505050505050505",
        "--claim-authority",
        "yes",
        "--active",
        "yes",
        "--request",
        "06060606060606060606060606060606",
        "--operation",
        "07070707070707070707070707070707",
        "--job",
        "08080808080808080808080808080808",
        "--action",
        "colony:1:0",
    ]);
    let Command::Step(step) = parse_command(&values).expect("explicit closed action") else {
        panic!("step")
    };
    assert_eq!(step.request, RequestId::from_bytes([6; 16]));
    assert_eq!(step.operation, OperationId::from_bytes([7; 16]));
    assert_eq!(step.job, JobId::from_bytes([8; 16]));
    assert!(step.active);
    assert_eq!(
        step.action,
        nf_world_driver::ActionSelection::Colony {
            site: 1,
            faction: 0
        }
    );
    for bad in [
        "colony:6:0",
        "build:1:3",
        "relationship:0:1:10001",
        "travel:0:3",
        "colony:1:0:extra",
        "unknown:1",
    ] {
        *values.last_mut().unwrap() = bad.into();
        assert!(parse_command(&values).is_err());
    }
}
#[test]
fn create_requires_explicit_pinned_public_membership_owner_and_controllers() {
    let values = args(&[
        "create",
        "--database",
        "private/world.sqlite",
        "--vault",
        "private/key",
        "--game-save-root",
        "saves",
        "--membership-file",
        "public.membership",
        "--membership-digest",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--universe",
        "01010101010101010101010101010101",
        "--history",
        "02020202020202020202020202020202",
        "--seed",
        "0303030303030303030303030303030303030303030303030303030303030303",
        "--controllers",
        "04040404040404040404040404040404,05050505050505050505050505050505,06060606060606060606060606060606",
        "--aggregate",
        "14141414141414141414141414141414",
        "--provider",
        "15151515151515151515151515151515",
        "--provider-aggregate",
        "16161616161616161616161616161616",
        "--owner-account",
        "04040404040404040404040404040404",
        "--owner-account-key",
        "0707070707070707070707070707070707070707070707070707070707070707",
        "--owner-device",
        "08080808080808080808080808080808",
        "--owner-device-key",
        "0909090909090909090909090909090909090909090909090909090909090909",
        "--owner-peer-hex",
        "0102",
    ]);
    let Command::Create(create) = parse_command(&values).expect("explicit trust root create")
    else {
        panic!("create")
    };
    assert_eq!(create.policy.membership_digest, [0xaa; 32]);
    assert_eq!(create.policy.owner.peer, vec![1, 2]);
    assert_eq!(create.genesis.genesis.accounts, create.policy.controllers);
    assert_eq!(create.genesis.genesis.seed, [3; 32]);
    assert!(parse_command(&values[..values.len() - 2]).is_err());
    assert!(parse_command(&args(&["create", "--database", "private/world.sqlite"])).is_err());
}
