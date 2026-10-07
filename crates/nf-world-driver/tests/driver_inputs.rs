mod driver_support;
use nf_world_driver::Driver;
#[test]
fn bounded_import_trust_root_and_missing_keys_fail_before_database_creation_or_private_output() {
    let f = driver_support::Fixture::new();
    let mut options = f.options.clone();
    options.policy.membership_digest[0] ^= 1;
    assert!(Driver::create(options).is_err());
    assert!(!f.options.database.exists());
    let large = f.scratch.0.join("oversize.membership");
    std::fs::File::create(&large)
        .unwrap()
        .set_len(262145)
        .unwrap();
    let mut options = f.options.clone();
    options.membership_file = large;
    assert_eq!(
        Driver::create(options).err(),
        Some(nf_world_driver::DriverError::Limit)
    );
    assert!(!f.options.database.exists());
    let mut options = f.options.clone();
    options.vault = f.scratch.0.join("absent");
    assert!(Driver::create(options).is_err());
    assert!(!f.options.database.exists());
    assert!(!f.scratch.0.join("absent").exists());
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    command.arg("--help");
    let output = driver_support::process::run(command);
    assert!(output.status.success());
    assert!(output.stdout.len() < 4096);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        nf_world_driver::command_help()
    );
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    command.args(["create", "--owner-account-key", "not-a-key"]);
    let output = driver_support::process::run(command);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("not-a-key")
    );
}
