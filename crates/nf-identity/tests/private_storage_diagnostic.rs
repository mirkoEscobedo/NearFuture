#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::{
    model::IdentityError,
    private_storage::{PrivateCause, PrivateOperation, PrivateStage, PrivateVault},
};
use std::io::ErrorKind;

#[test]
fn duplicate_identity_returns_create_new_cause_and_retains_original() {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    std::fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    let original = vault.create_identity(vec![1]).unwrap();
    let failure = vault.create_identity_detailed(vec![1]).err().unwrap();
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    assert_eq!(
        failure.diagnostic().map(|d| d.operation()),
        Some(PrivateOperation::IdentityCreate)
    );
    assert_eq!(
        failure.diagnostic().map(|d| d.stage()),
        Some(PrivateStage::CreateNew)
    );
    assert!(matches!(
        failure.diagnostic().map(|d| d.cause()),
        Some(PrivateCause::Io {
            kind: ErrorKind::AlreadyExists,
            ..
        })
    ));
    let retained = vault.load_identity(vec![1]).unwrap();
    assert!(
        retained.public == original.public,
        "original identity changed"
    );
}
#[test]
fn missing_identity_root_retains_single_acl_metadata_cause() {
    #[cfg(windows)]
    use nf_identity::private_storage::{HelperCause, HelperKind, HelperStep};
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    std::fs::create_dir(&saves).unwrap();
    let private = root.join("private");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    std::fs::remove_dir(&private).unwrap();
    let failure = vault.create_identity_detailed(vec![1]).err().unwrap();
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    let diagnostic = failure
        .diagnostic()
        .expect("root-access diagnostic missing");
    assert_eq!(diagnostic.operation(), PrivateOperation::IdentityCreate);
    assert_eq!(diagnostic.stage(), PrivateStage::RootAccess);
    #[cfg(windows)]
    assert!(matches!(diagnostic.cause(), PrivateCause::Helper(helper)
        if helper.kind() == HelperKind::SingleAcl && helper.step() == HelperStep::Metadata
        && matches!(helper.cause(), HelperCause::Io { kind: ErrorKind::NotFound, .. })));
    #[cfg(not(windows))]
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: ErrorKind::NotFound,
            ..
        }
    ));
}
#[test]
fn duplicate_blob_returns_create_new_cause_and_retains_payload() {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    std::fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    vault.create_private_blob("original", b"fixture").unwrap();
    let failure = vault
        .create_private_blob_detailed("original", b"replacement")
        .unwrap_err();
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    let diagnostic = failure.diagnostic().expect("create-new diagnostic missing");
    assert_eq!(diagnostic.operation(), PrivateOperation::BlobCreate);
    assert_eq!(diagnostic.stage(), PrivateStage::CreateNew);
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: ErrorKind::AlreadyExists,
            ..
        }
    ));
    assert!(
        vault.read_private_blob("original").unwrap() == b"fixture",
        "original payload changed"
    );
}
#[test]
fn missing_scan_root_retains_metadata_cause_without_visiting() {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    std::fs::create_dir(&saves).unwrap();
    let private = root.join("private");
    let vault = PrivateVault::create(&private, &saves).unwrap();
    std::fs::remove_dir(&private).unwrap();
    let mut visited = false;
    let failure = vault
        .scan_optional_private_blobs_detailed(&["original"], |_, _| {
            visited = true;
            Ok(())
        })
        .unwrap_err();
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    let diagnostic = failure
        .diagnostic()
        .expect("scan metadata diagnostic missing");
    assert_eq!(diagnostic.operation(), PrivateOperation::BlobScan);
    assert_eq!(diagnostic.stage(), PrivateStage::RootMetadata);
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: ErrorKind::NotFound,
            ..
        }
    ));
    assert!(!visited);
}
