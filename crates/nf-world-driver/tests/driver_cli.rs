mod driver_support;
use std::process::Command;
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|n| format!("{n:02x}")).collect()
}
#[test]
fn foreground_cli_creates_real_pinned_genesis_and_read_only_status_without_private_output() {
    let fixture = driver_support::Fixture::new();
    let o = &fixture.options;
    let mut create = Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    create
        .arg("create")
        .arg("--database")
        .arg(&o.database)
        .arg("--vault")
        .arg(&o.vault)
        .arg("--game-save-root")
        .arg(&o.game_save_root)
        .arg("--membership-file")
        .arg(&o.membership_file);
    create.args([
        "--membership-digest",
        &hex(&o.policy.membership_digest),
        "--universe",
        &hex(o.policy.scope.universe.as_bytes()),
        "--history",
        &hex(o.policy.scope.history.as_bytes()),
        "--seed",
        &hex(&o.genesis.genesis.seed),
        "--controllers",
        &o.policy
            .controllers
            .iter()
            .map(|id| hex(id.as_bytes()))
            .collect::<Vec<_>>()
            .join(","),
        "--aggregate",
        &hex(o.genesis.aggregate.as_bytes()),
        "--provider",
        &hex(o.genesis.provider.as_bytes()),
        "--provider-aggregate",
        &hex(o.genesis.provider_aggregate.as_bytes()),
        "--owner-account",
        &hex(o.policy.owner.account.as_bytes()),
        "--owner-account-key",
        &hex(&o.policy.owner.account_key),
        "--owner-device",
        &hex(o.policy.owner.device.as_bytes()),
        "--owner-device-key",
        &hex(&o.policy.owner.device_key),
        "--owner-peer-hex",
        &hex(&o.policy.owner.peer),
    ]);
    let output = driver_support::process::run(create);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = String::from_utf8(output.stdout).unwrap();
    assert!(first.contains("tick=0\n"));
    assert!(first.contains("membership_revision=2\n"));
    assert!(first.contains("authority_term=0\n"));
    assert!(first.len() < 4096);
    let mut status = Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    status
        .arg("status")
        .arg("--database")
        .arg(&o.database)
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
        ]);
    let output = driver_support::process::run(status);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), first);
    assert_eq!(std::fs::read_dir(&o.game_save_root).unwrap().count(), 0);
}
