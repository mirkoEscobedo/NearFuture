use nf_contract::identity::{AuthorityTerm, EventSeq, HistoryId, UniverseId};
use nf_store::miniature::IsolatedRestoreConfirmation;
use nf_world_driver::{Command, CommandError, command_help, parse_command};

fn args() -> Vec<String> {
    [
        "restore-isolated",
        "--backup",
        "public/backup.sqlite",
        "--destination",
        "isolated/new.sqlite",
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
        "--confirm",
        "NewReadOnlyCopy",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[test]
fn isolated_restore_admits_only_explicit_copy_with_each_independently_held_minimum() {
    let values = args();
    let Command::RestoreIsolated(options) = parse_command(&values).unwrap() else {
        panic!("isolated restore variant required")
    };
    assert_eq!(options.backup.to_str(), Some("public/backup.sqlite"));
    assert_eq!(options.destination.to_str(), Some("isolated/new.sqlite"));
    assert_eq!(
        options.protected.storage.scope.universe,
        UniverseId::from_bytes([1; 16])
    );
    assert_eq!(
        options.protected.storage.scope.history,
        HistoryId::from_bytes([2; 16])
    );
    assert_eq!(options.protected.storage.event_sequence, EventSeq(7));
    assert_eq!(options.protected.storage.store_revision, 9);
    assert_eq!(options.protected.storage.membership_revision, Some(3));
    assert_eq!(options.protected.minimum_authority_term, AuthorityTerm(2));
    assert_eq!(
        options.confirmation,
        IsolatedRestoreConfirmation::NewReadOnlyCopy
    );
    for index in (1..values.len()).step_by(2) {
        let mut missing = values.clone();
        missing.drain(index..index + 2);
        assert_eq!(parse_command(&missing), Err(CommandError::Missing));
    }
}

#[test]
fn isolated_restore_refuses_implicit_confirmation_owner_flags_and_noncanonical_minima() {
    for invalid in ["yes", "no", "NewReadonlyCopy", "new_read_only_copy"] {
        let mut values = args();
        *values.last_mut().unwrap() = invalid.into();
        assert_eq!(parse_command(&values), Err(CommandError::Invalid));
    }
    for (key, value) in [
        ("--active", "yes"),
        ("--claim-authority", "yes"),
        ("--vault", "private/key"),
        ("--overwrite", "yes"),
        ("--create-parent", "yes"),
        ("--adopt", "yes"),
        ("--min-event", "7"),
    ] {
        let mut values = args();
        values.extend([key.to_owned(), value.to_owned()]);
        assert_eq!(parse_command(&values), Err(CommandError::Invalid));
    }
    for key in ["--min-event", "--min-store", "--min-membership", "--min-term"] {
        for invalid in ["01", "+1", "-1", "18446744073709551616"] {
            let mut values = args();
            let index = values.iter().position(|value| value == key).unwrap() + 1;
            values[index] = invalid.into();
            assert_eq!(parse_command(&values), Err(CommandError::Invalid));
        }
    }
    assert!(command_help().len() <= 4096);
    assert!(command_help().contains("--confirm NewReadOnlyCopy"));
    assert!(command_help().contains("unqualified partial copy; no cleanup or retry"));
}
