use super::{
    CASE_LIMIT, DATABASE_LIMIT, OwnedChild, RestoreDatabasePermissions, STDERR_LIMIT, STDOUT_LIMIT,
    binary_digest, bounded, directory_names, finish_child, fixture, hex, new_file, status_options,
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Instant,
};

#[test]
fn support_preview_is_explicit_closed_redacted_data_from_read_only_profile2() {
    let deadline = Instant::now() + CASE_LIMIT;
    let executable = Path::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    let expected_binary = binary_digest(executable, deadline);
    let fixture = fixture(deadline);
    let before_database = bounded(&fixture.database, DATABASE_LIMIT);
    let capture_root = fixture.root.join("support-preview-captures");
    fs::create_dir(&capture_root).unwrap();
    let stdout = capture_root.join("preview.stdout.raw");
    let stderr = capture_root.join("preview.stderr.raw");
    let out = new_file(&stdout);
    let err = new_file(&stderr);
    out.sync_all().unwrap();
    err.sync_all().unwrap();
    let before_directory = directory_names(&fixture.root);
    let permissions = fs::metadata(&fixture.database).unwrap().permissions();
    let restore = RestoreDatabasePermissions(fixture.database.clone(), permissions.clone());
    let mut read_only = permissions;
    read_only.set_readonly(true);
    fs::set_permissions(&fixture.database, read_only).unwrap();

    let reader =
        nf_world_driver::Driver::read_only_status(status_options(&fixture, fixture.known)).unwrap();
    assert!(reader.is_read_only().unwrap());
    assert_eq!(reader.snapshot(), &fixture.snapshot);
    let before_state = reader.snapshot_bytes().unwrap();
    drop(reader);

    let mut command = Command::new(executable);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .arg("support-preview")
        .arg("--database")
        .arg(&fixture.database)
        .args([
            "--universe",
            &hex(fixture.known.storage.scope.universe.as_bytes()),
            "--history",
            &hex(fixture.known.storage.scope.history.as_bytes()),
            "--min-event",
            "0",
            "--min-store",
            "0",
            "--min-membership",
            "2",
            "--min-term",
            "0",
        ]);
    assert!(Instant::now() < deadline, "before support preview child deadline");
    let mut owned = OwnedChild {
        child: command.spawn().expect("one owned support preview CLI child"),
        reaped: false,
        termination_requested: false,
        cleanup_deadline: None,
        cleanup_receipt: capture_root.join("cleanup.public"),
    };
    drop(command);
    let status = finish_child(&mut owned, &stdout, &stderr, deadline);
    drop(owned);
    // Read both complete bounded captures before the missing-behavior assertion.
    let output = bounded(&stdout, STDOUT_LIMIT);
    let errors = bounded(&stderr, STDERR_LIMIT);
    // Independent complete byte oracle, including actual pinned Cargo version.
    let expected_output = concat!(
        "{\"format\":\"NF-OPERATOR-SUPPORT-PREVIEW-1\",\"coverage\":\"miniature_status_only\",",
        "\"software\":{\"package\":\"nf-world-driver\",\"version\":\"0.1.0\",\"source_revision\":null},",
        "\"files\":[{\"name\":\"status.json\",\"media_type\":\"application/json\",\"content\":",
        "{\"format\":\"NF-OPERATOR-MINIATURE-STATUS-1\",\"store_profile\":\"profile2\",",
        "\"world_tick\":\"0\",\"event_sequence\":\"0\",\"store_revision\":\"0\",",
        "\"membership_revision\":\"2\",\"persisted_authority_term\":null,",
        "\"minimum_known_authority_term\":\"0\",\"provider_revision\":\"0\",",
        "\"pending_frontier\":false,\"pending_intents_count\":\"0\",",
        "\"provider_ownership\":\"unavailable\",\"lag\":\"unavailable\",",
        "\"taint\":\"unavailable\",\"pending_native_bridge\":\"unavailable\",",
        "\"transfer_receipts\":\"unavailable_profile2\",\"backup_recovery\":\"unavailable\"}}],",
        "\"excluded\":[\"keys\",\"private_chat\",\"save_content\",\"sensitive_paths\"],",
        "\"unavailable\":[\"logs\",\"manifests\",\"transfers\",\"backup_recovery\"]}",
    )
    .as_bytes();
    let output_matches = output == expected_output;
    let stderr_empty = errors.is_empty();
    let unsupported_command_observed = status.code() == Some(1)
        && output.is_empty()
        && errors == b"driver: InvalidCommand\n";
    assert_eq!(
        status.code(),
        Some(0),
        "support-preview must succeed; output_matches={output_matches}; stderr_empty={stderr_empty}; unsupported_command_observed={unsupported_command_observed}; captures retained"
    );
    assert!(stderr_empty, "unexpected support preview stderr retained");
    assert_eq!(output, expected_output);
    assert!(!output.windows(fixture.root.as_os_str().to_string_lossy().len()).any(|bytes| {
        bytes == fixture.root.as_os_str().to_string_lossy().as_bytes()
    }));
    assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);
    assert_eq!(directory_names(&fixture.root), before_directory);
    assert!(fs::metadata(&fixture.database).unwrap().permissions().readonly());

    let reader =
        nf_world_driver::Driver::read_only_status(status_options(&fixture, fixture.known)).unwrap();
    assert!(reader.is_read_only().unwrap());
    assert_eq!(reader.snapshot(), &fixture.snapshot);
    assert_eq!(reader.snapshot_bytes().unwrap(), before_state);
    drop(reader);
    assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);
    assert_eq!(directory_names(&fixture.root), before_directory);
    assert_eq!(binary_digest(executable, deadline), expected_binary);
    drop(restore);
    assert!(Instant::now() < deadline, "complete support preview case deadline");
}
