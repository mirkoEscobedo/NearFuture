// A reader inspects a quiescent existing database without obtaining write access.
mod miniature_support;
use miniature_support::{Fixture, Scratch, claim};
use nf_store::{
    StoreError,
    miniature::{MiniatureReader, MiniatureStore, MiniatureStoreError},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct RestorePermissions(PathBuf, fs::Permissions);
impl Drop for RestorePermissions {
    fn drop(&mut self) {
        fs::set_permissions(&self.0, self.1.clone())
            .expect("restore disposable database permissions");
    }
}
fn entries(root: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    names
}

#[test]
fn read_only_profile2_snapshot_preserves_bytes_and_rejects_newer_minima() {
    let scratch = Scratch::new();
    let mut f = Fixture::new();
    let mut writer = MiniatureStore::create(
        scratch.db(),
        f.spec,
        &f.bytes(),
        f.policy.clone(),
        &mut f.signer,
    )
    .unwrap();
    claim(&mut writer, &mut f);
    let expected = writer.snapshot().unwrap();
    let state_bytes = writer.snapshot_bytes().unwrap();
    let known = expected.known;
    assert_eq!(known.storage.event_sequence.0, 0);
    assert_eq!(known.storage.store_revision, 1);
    assert_eq!(known.storage.membership_revision, Some(2));
    assert_eq!(known.minimum_authority_term.0, 1);
    drop(writer); // Existing writer/DELETE journal must be quiescent before inspection.
    let before = fs::read(scratch.db()).unwrap();
    let names = entries(&scratch.0);
    assert_eq!(names, vec![std::ffi::OsString::from("history.sqlite")]);
    let original = fs::metadata(scratch.db()).unwrap().permissions();
    let restore = RestorePermissions(scratch.db(), original.clone());
    let mut read_only = original;
    read_only.set_readonly(true);
    fs::set_permissions(scratch.db(), read_only).unwrap();

    let reader = MiniatureReader::open_existing(scratch.db(), known).unwrap();
    // This getter forwards SQLite's actual main-database read-only state.
    assert!(reader.is_read_only().unwrap());
    assert_eq!(reader.snapshot(), &expected);
    assert_eq!(reader.snapshot_bytes().unwrap(), state_bytes);
    drop(reader);
    assert_eq!(fs::read(scratch.db()).unwrap(), before);
    assert_eq!(entries(&scratch.0), names);

    for field in 0..4 {
        let mut newer = known;
        match field {
            0 => newer.storage.event_sequence = nf_contract::identity::EventSeq(1),
            1 => newer.storage.store_revision = 2,
            2 => newer.storage.membership_revision = Some(3),
            _ => newer.minimum_authority_term = nf_contract::identity::AuthorityTerm(2),
        }
        assert_eq!(
            MiniatureReader::open_existing(scratch.db(), newer).err(),
            Some(MiniatureStoreError::Storage(StoreError::StaleBackup))
        );
        assert_eq!(fs::read(scratch.db()).unwrap(), before);
        assert_eq!(entries(&scratch.0), names);
    }
    let reader = MiniatureReader::open_existing(scratch.db(), known).unwrap();
    assert_eq!(reader.snapshot(), &expected);
    assert_eq!(reader.snapshot_bytes().unwrap(), state_bytes);
    drop(reader);
    drop(restore);
}
