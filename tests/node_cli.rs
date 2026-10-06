use std::process::Command;

#[test]
fn synthetic_node_identifies_its_build_without_game_claims() {
    let output = Command::new(env!("CARGO_BIN_EXE_near-future"))
        .arg("--version")
        .output()
        .expect("node must start");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Near Future 0.1.0 (synthetic; game integration unavailable)\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn unknown_arguments_fail_without_claiming_a_running_node() {
    let output = Command::new(env!("CARGO_BIN_EXE_near-future"))
        .arg("--connect")
        .output()
        .expect("node must start");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Usage: near-future [--version]\n"
    );
}
