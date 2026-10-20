//! An isolated restored copy is inspectable data, never activation authority.
mod miniature_support;
use miniature_support::{Fixture, Scratch, claim};
use nf_contract::identity::HistoryId;
use nf_identity::model::MembershipRepository;
use nf_kernel::miniature::settle_miniature;
use nf_store::{
    StoreError,
    miniature::{
        BootstrapSigner, ChallengeRequest, IsolatedRestoreConfirmation, MiniatureReader,
        MiniatureStore, MiniatureStoreError, ProofAttempt,
    },
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn entries(path: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    names
}

fn assert_public_membership(path: &Path, expected: &[u8]) {
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    assert!(
        connection
            .is_readonly(rusqlite::DatabaseName::Main)
            .unwrap()
    );
    let count: i64 = connection
        .query_row("SELECT count(*) FROM membership", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let (body, digest): (Vec<u8>, Vec<u8>) = connection
        .query_row("SELECT public_state,digest FROM membership", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(body, expected);
    assert_eq!(digest, Sha256::digest(expected).to_vec());
}

#[test]
fn isolated_backup_restore_preserves_originals_and_refuses_newer_known_same_history() {
    let scratch = Scratch::new();
    let mut fixture = Fixture::new();
    let mut writer = MiniatureStore::create(
        scratch.db(),
        fixture.spec,
        &fixture.bytes(),
        fixture.policy.clone(),
        &mut fixture.signer,
    )
    .unwrap();
    claim(&mut writer, &mut fixture);
    let expected = writer.snapshot().unwrap();
    let expected_state = writer.snapshot_bytes().unwrap();
    let known = writer.known_frontiers().unwrap();
    let expected_membership = fixture.bytes();
    let backup = scratch.0.join("known-backup.sqlite");
    assert_eq!(writer.backup_to(&backup).unwrap(), known);
    drop(writer);
    assert!(fs::metadata(&backup).unwrap().len() <= 1_048_576);
    let backup_before = fs::read(&backup).unwrap();
    let source_before = fs::read(scratch.db()).unwrap();
    let isolated = scratch.0.join("isolated-restoration");
    fs::create_dir(&isolated).unwrap();
    let restored = isolated.join("verified-copy.sqlite");
    assert!(!restored.exists());

    let reader = MiniatureReader::restore_backup_to_new_path(
        &backup,
        &restored,
        known,
        IsolatedRestoreConfirmation::NewReadOnlyCopy,
    )
    .unwrap();
    assert!(restored.is_file());
    assert!(reader.is_read_only().unwrap());
    assert_eq!(reader.snapshot(), &expected);
    assert_eq!(reader.snapshot_bytes().unwrap(), expected_state);
    drop(reader);
    assert_public_membership(&restored, &expected_membership);
    let reopened = MiniatureReader::open_existing(&restored, known).unwrap();
    assert!(reopened.is_read_only().unwrap());
    assert_eq!(reopened.snapshot(), &expected);
    assert_eq!(reopened.snapshot_bytes().unwrap(), expected_state);
    drop(reopened);
    let restored_before = fs::read(&restored).unwrap();
    let isolated_before = entries(&isolated);
    assert_eq!(fs::read(&backup).unwrap(), backup_before);
    assert_eq!(fs::read(scratch.db()).unwrap(), source_before);
    assert_eq!(
        MiniatureReader::restore_backup_to_new_path(
            &backup,
            &restored,
            known,
            IsolatedRestoreConfirmation::NewReadOnlyCopy,
        )
        .err(),
        Some(MiniatureStoreError::Storage(StoreError::AlreadyExists))
    );
    assert_eq!(fs::read(&restored).unwrap(), restored_before);

    // Only this explicit live-owner fixture flow advances the original history.
    let mut writer =
        MiniatureStore::open_existing(scratch.db(), known, fixture.policy.auth).unwrap();
    claim(&mut writer, &mut fixture);
    writer.prepare(vec![], vec![]).unwrap();
    let issued = writer
        .issue_challenge(ChallengeRequest::Activity {
            actor: fixture.policy.owner.account,
            device: fixture.policy.owner.device,
        })
        .unwrap();
    let proof = fixture.signer.sign(&issued.template).unwrap();
    writer
        .accept_activity(ProofAttempt {
            ticket: issued.ticket,
            proof,
        })
        .unwrap();
    let batch = settle_miniature(
        writer.world(),
        writer.pending().unwrap(),
        writer.authority().unwrap().context(),
    )
    .unwrap();
    let issued = writer.issue_challenge(ChallengeRequest::Commit).unwrap();
    let proof = fixture.signer.sign(&issued.template).unwrap();
    writer
        .commit(
            &batch,
            ProofAttempt {
                ticket: issued.ticket,
                proof,
            },
        )
        .unwrap();
    let mut membership = fixture.membership.clone();
    membership.revision += 1;
    membership.consumed.insert([99; 16]);
    writer
        .commit_membership(Some(fixture.membership.revision), &membership)
        .unwrap();
    claim(&mut writer, &mut fixture);
    let newer = writer.known_frontiers().unwrap();
    let newest = writer.snapshot().unwrap();
    let newest_state = writer.snapshot_bytes().unwrap();
    assert!(newer.storage.event_sequence > known.storage.event_sequence);
    assert!(newer.storage.store_revision > known.storage.store_revision);
    assert!(newer.storage.membership_revision > known.storage.membership_revision);
    assert!(newer.minimum_authority_term > known.minimum_authority_term);
    drop(writer);
    let newest_source_before = fs::read(scratch.db()).unwrap();

    for field in 0..4 {
        let mut protected = known;
        match field {
            0 => protected.storage.event_sequence = newer.storage.event_sequence,
            1 => protected.storage.store_revision = newer.storage.store_revision,
            2 => protected.storage.membership_revision = newer.storage.membership_revision,
            _ => protected.minimum_authority_term = newer.minimum_authority_term,
        }
        let target = isolated.join(format!("stale-{field}.sqlite"));
        assert!(!target.exists());
        assert_eq!(
            MiniatureReader::restore_backup_to_new_path(
                &backup,
                &target,
                protected,
                IsolatedRestoreConfirmation::NewReadOnlyCopy,
            )
            .err(),
            Some(MiniatureStoreError::Storage(StoreError::StaleBackup))
        );
        assert!(!target.exists(), "refusal must precede target creation");
        assert_eq!(entries(&isolated), isolated_before);
    }
    let mut other_history = known;
    other_history.storage.scope.history = HistoryId::from_bytes([99; 16]);
    let target = isolated.join("wrong-history.sqlite");
    assert_eq!(
        MiniatureReader::restore_backup_to_new_path(
            &backup,
            &target,
            other_history,
            IsolatedRestoreConfirmation::NewReadOnlyCopy,
        )
        .err(),
        Some(MiniatureStoreError::Storage(StoreError::Scope))
    );
    assert!(!target.exists());
    assert_eq!(entries(&isolated), isolated_before);
    assert_eq!(fs::read(&backup).unwrap(), backup_before);
    assert_eq!(fs::read(&restored).unwrap(), restored_before);
    assert_eq!(fs::read(scratch.db()).unwrap(), newest_source_before);
    let current = MiniatureReader::open_existing(scratch.db(), newer).unwrap();
    assert!(current.is_read_only().unwrap());
    assert_eq!(current.snapshot(), &newest);
    assert_eq!(current.snapshot_bytes().unwrap(), newest_state);
}
