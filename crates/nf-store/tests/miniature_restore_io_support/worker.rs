use super::{bounded, create, epoch_millis};
use nf_contract::identity::{AuthorityTerm, EventSeq, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_store::{
    KnownFrontiers, StoreError,
    miniature::{IsolatedRestoreConfirmation, MiniatureKnownFrontiers, MiniatureReader, MiniatureStoreError},
};
use std::{io::Write, path::Path, thread, time::Duration};

pub fn known_bytes(known: MiniatureKnownFrontiers) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(known.storage.scope.universe.as_bytes());
    bytes.extend(known.storage.scope.history.as_bytes());
    for value in [
        known.storage.event_sequence.0, known.storage.store_revision,
        known.storage.membership_revision.expect("maintained fixture membership"),
        known.minimum_authority_term.0,
    ] {
        bytes.extend(value.to_le_bytes());
    }
    assert_eq!(bytes.len(), 64);
    bytes
}

fn known(path: &Path) -> MiniatureKnownFrontiers {
    let bytes = bounded(path, 64);
    assert_eq!(bytes.len(), 64);
    let value = |start| u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
    MiniatureKnownFrontiers {
        storage: KnownFrontiers {
            scope: Scope {
                universe: UniverseId::from_slice(&bytes[..16]).unwrap(),
                history: HistoryId::from_slice(&bytes[16..32]).unwrap(),
            },
            event_sequence: EventSeq(value(32)),
            store_revision: value(40),
            membership_revision: Some(value(48)),
        },
        minimum_authority_term: AuthorityTerm(value(56)),
    }
}

pub fn run() {
    let Some(mode) = std::env::var_os("NF_RESTORE_IO_WORKER") else {
        return;
    };
    let mode = mode.to_str().unwrap();
    assert!(matches!(mode, "calibration" | "fault"));
    let destination = match mode {
        "calibration" => "calibration.sqlite",
        "fault" => "fault.sqlite",
        _ => unreachable!(),
    };
    let control = Path::new("control").join(mode);
    let known = known(Path::new("control/protected-known.bin"));
    let expected = bounded(Path::new("control/expected-state.bin"), 1_048_576);
    let deadline: u128 = std::env::var("NF_RESTORE_IO_DEADLINE_MS").unwrap().parse().unwrap();
    assert!(epoch_millis() < deadline);
    let actual_thread = std::fs::read_link("/proc/thread-self").unwrap();
    let tid: u32 = actual_thread.file_name().unwrap().to_str().unwrap().parse().unwrap();
    create(&control.join("ready.txt"), format!("{} {tid}\n", std::process::id()).as_bytes());
    while !control.join("release.txt").exists() {
        assert!(epoch_millis() < deadline, "same parent-owned absolute deadline");
        thread::sleep(Duration::from_millis(10));
    }
    assert!(epoch_millis() < deadline);
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(b"RESTORE_CALL_READY\n").unwrap();
    stdout.flush().unwrap();
    let result = MiniatureReader::restore_backup_to_new_path(
        "backup.sqlite", destination, known, IsolatedRestoreConfirmation::NewReadOnlyCopy,
    );
    match mode {
        "calibration" => {
            let reader = result.expect("real restore calibration");
            assert!(reader.is_read_only().unwrap());
            assert_eq!(reader.snapshot().known, known);
            assert_eq!(reader.snapshot_bytes().unwrap(), expected);
            drop(reader);
            stdout.write_all(b"RESTORE_QUALIFIED_ACK\n").unwrap();
            stdout.flush().unwrap();
            create(&control.join("result.txt"), b"QUALIFIED_READER\n");
        }
        "fault" => {
            assert_eq!(result.err(), Some(MiniatureStoreError::Storage(StoreError::Io)));
            stdout.write_all(b"RESTORE_REFUSED_IO\n").unwrap();
            stdout.flush().unwrap();
            create(&control.join("result.txt"), b"REFUSED_STORAGE_IO\n");
        }
        _ => unreachable!(),
    }
    assert!(epoch_millis() < deadline);
}
