//! Actual executable restoration is isolated inspection, never active adoption.
#[path = "../../nf-store/tests/miniature_support/mod.rs"]
mod miniature_support;
#[path = "driver_isolated_restore_cli/support.rs"]
mod support;
use nf_store::miniature::MiniatureReader;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use support::{Entry, Fixture, tree};

const SUCCESS: &[u8] = concat!(
    "{\"format\":\"NF-OPERATOR-ISOLATED-RESTORE-1\",",
    "\"result\":\"new_read_only_copy\",\"store_profile\":\"profile2\",",
    "\"activation\":\"unavailable\"}"
)
.as_bytes();

#[test]
#[ignore = "requires a fresh caller-owned retained public CLI evidence parent"]
fn actual_isolated_restore_cli_preserves_originals_and_all_independent_minima() {
    let deadline = Instant::now() + Duration::from_secs(20);
    let fixture = Fixture::new(deadline);
    let executable = Path::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    let binary = support::binary_digest(executable, deadline);
    support::receipt(&fixture.control.join("binary.public.sha256"), &binary);
    let _ = tree(&fixture.control, deadline);
    let before = tree(&fixture.data, deadline);
    let destination = fixture.data.join("inspection.sqlite");
    let result = support::run(
        executable,
        &fixture,
        "restore",
        &destination,
        fixture.known,
        true,
        deadline,
    );
    // The immutable parent has no restore-isolated verb: actual exit1 and
    // InvalidCommand are retained, and this is the first intended behavior RED.
    assert_eq!(
        result.status.code(),
        Some(0),
        "actual CLI restoration failed; original captures retained"
    );
    assert_eq!(result.stdout, SUCCESS);
    assert!(result.stderr.is_empty());
    let _ = tree(&fixture.control, deadline);
    let reader = MiniatureReader::open_existing(&destination, fixture.known).unwrap();
    assert!(reader.is_read_only().unwrap());
    assert_eq!(reader.snapshot(), &fixture.snapshot);
    assert_eq!(reader.snapshot_bytes().unwrap(), fixture.state);
    drop(reader); // No fallible public close API; no successful-close receipt claimed.
    fixture.assert_membership(&destination, deadline);
    fixture.assert_membership(&fixture.backup, deadline);
    let after = tree(&fixture.data, deadline);
    let mut expected = before.clone();
    let restored = after.get(Path::new("inspection.sqlite")).unwrap().clone();
    assert!(matches!(restored, Entry::File(_)));
    assert!(
        expected
            .insert("inspection.sqlite".into(), restored)
            .is_none()
    );
    assert_eq!(
        after, expected,
        "one isolated file is the only data-tree effect"
    );
    fixture.assert_original(deadline);

    for index in 0..6 {
        let mut protected = fixture.known;
        let (target, confirmed, diagnostic) = match index {
            0 => (
                fixture.data.join("unconfirmed.sqlite"),
                false,
                b"driver: InvalidCommand\n".as_slice(),
            ),
            1 => (
                destination.clone(),
                true,
                b"driver: Store(Storage(AlreadyExists))\n".as_slice(),
            ),
            _ => {
                match index {
                    2 => protected.storage.event_sequence = fixture.newer.storage.event_sequence,
                    3 => protected.storage.store_revision = fixture.newer.storage.store_revision,
                    4 => {
                        protected.storage.membership_revision =
                            fixture.newer.storage.membership_revision
                    }
                    _ => protected.minimum_authority_term = fixture.newer.minimum_authority_term,
                }
                (
                    fixture.data.join(format!("stale-{index}.sqlite")),
                    true,
                    b"driver: Store(Storage(StaleBackup))\n".as_slice(),
                )
            }
        };
        let held = tree(&fixture.data, deadline);
        let result = support::run(
            executable,
            &fixture,
            &format!("refusal-{index}"),
            &target,
            protected,
            confirmed,
            deadline,
        );
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, diagnostic);
        let _ = tree(&fixture.control, deadline);
        assert_eq!(
            tree(&fixture.data, deadline),
            held,
            "whole-tree refusal precedes every output effect"
        );
        fixture.assert_original(deadline);
        let reader = MiniatureReader::open_existing(&destination, fixture.known).unwrap();
        assert!(reader.is_read_only().unwrap());
        assert_eq!(reader.snapshot(), &fixture.snapshot);
        assert_eq!(reader.snapshot_bytes().unwrap(), fixture.state);
        drop(reader);
        fixture.assert_membership(&destination, deadline);
        fixture.assert_membership(&fixture.backup, deadline);
        assert_eq!(tree(&fixture.data, deadline), held);
    }
    assert_eq!(support::binary_digest(executable, deadline), binary);
    support::receipt(
        &fixture.control.join("success.public"),
        "seven exact CLI outcomes; originals and data tree preserved; direct children actually reaped; no activation claim",
    );
    let _ = tree(&fixture.control, deadline);
    assert!(
        Instant::now() < deadline,
        "single complete CLI case deadline"
    );
}
