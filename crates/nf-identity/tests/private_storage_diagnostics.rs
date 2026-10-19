//! Closed failure context is observable without exposing the target path.
use nf_identity::{
    model::IdentityError,
    private_storage::{PrivateStorageReason, PrivateStorageStage, PrivateVault},
};
use std::{fs, io, path::PathBuf};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}

#[test]
fn existing_directory_failure_has_closed_stage_and_preserves_legacy_error() {
    let root = std::env::temp_dir().join(format!(
        "nf-storage-diagnostic-private-sentinel-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let scratch = Scratch(root);
    let failure = match PrivateVault::create_diagnosed(&scratch.0, &scratch.0) {
        Err(failure) => failure,
        Ok(_) => panic!("existing directory was admitted"),
    };
    assert_eq!(failure.stage, PrivateStorageStage::CreateDirectory);
    assert_eq!(failure.reason, PrivateStorageReason::Io);
    assert_eq!(failure.io.unwrap().kind, io::ErrorKind::AlreadyExists);
    assert!(failure.helper.is_none());
    assert_eq!(
        PrivateVault::create(&scratch.0, &scratch.0).err(),
        Some(IdentityError::PrivateStorage)
    );
    let rendered = format!("{failure:?}");
    assert!(!rendered.contains("private-sentinel"));
    let target = scratch.0.to_string_lossy();
    assert!(!rendered.contains(target.as_ref()));
}
