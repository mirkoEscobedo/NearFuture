use nf_world_driver::{Command, parse_command};

#[test]
fn explicit_redacted_status_is_admitted_with_protected_minima_without_an_action_variant() {
    let values: Vec<String> = [
        "status-redacted",
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
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let command = parse_command(&values).expect("explicit redacted status admission");
    assert!(!matches!(
        command,
        Command::Create(_) | Command::Step(_) | Command::Run(_)
    ));
    assert!(parse_command(&values[..values.len() - 2]).is_err());
    let mut action_flags = values;
    action_flags.extend(["--active".to_owned(), "yes".to_owned()]);
    assert!(parse_command(&action_flags).is_err());
}
