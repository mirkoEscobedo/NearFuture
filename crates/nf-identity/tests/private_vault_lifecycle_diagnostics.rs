use nf_identity::{
    model::IdentityError,
    private_storage::{PrivateCause, PrivateOperation, PrivateStage, PrivateVault},
};
use std::{fs, io::ErrorKind, path::PathBuf};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let mut random = [0; 16];
        getrandom::fill(&mut random).unwrap();
        let suffix = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .join(format!("private-vault-lifecycle-{suffix}"));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("saves")).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        assert!(self.0.starts_with(parent));
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn vault_creation_retains_the_failed_directory_operation_without_path_data() {
    let scratch = Scratch::new();
    let existing = scratch.0.join("private-path-must-stay-out-of-diagnostics");
    fs::create_dir(&existing).unwrap();
    let failure = PrivateVault::create_detailed(&existing, &scratch.0.join("saves"))
        .err()
        .expect("existing directory must refuse creation");
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    let diagnostic = failure
        .diagnostic()
        .expect("actual failed vault creation must retain typed evidence");
    assert_eq!(diagnostic.operation(), PrivateOperation::VaultCreate);
    assert_eq!(diagnostic.stage(), PrivateStage::CreateDirectory);
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: ErrorKind::AlreadyExists,
            ..
        }
    ));
    assert!(!format!("{failure:?}").contains("private-path-must-stay-out-of-diagnostics"));
    assert_eq!(
        PrivateVault::create(&existing, &scratch.0.join("saves")).err(),
        Some(IdentityError::PrivateStorage),
    );
}

#[test]
fn vault_open_preserves_canonicalization_failure_and_save_exclusion() {
    let scratch = Scratch::new();
    let valid = scratch.0.join("valid-private-root");
    let saves = scratch.0.join("saves");
    PrivateVault::create_detailed(&valid, &saves).unwrap_or_else(|e| panic!("{e:?}"));
    PrivateVault::open(&valid, &saves).unwrap_or_else(|e| panic!("{e:?}"));
    let absent = scratch.0.join("absent-game-root");
    let failure = PrivateVault::open_detailed(&valid, &absent)
        .err()
        .expect("missing game root");
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    let diagnostic = failure
        .diagnostic()
        .expect("same-call canonicalization evidence");
    assert_eq!(diagnostic.operation(), PrivateOperation::VaultOpen);
    assert_eq!(diagnostic.stage(), PrivateStage::SaveRootCanonicalize);
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: ErrorKind::NotFound,
            ..
        }
    ));
    assert_eq!(
        PrivateVault::open(&valid, &absent).err(),
        Some(IdentityError::PrivateStorage)
    );
    let failure = PrivateVault::create_detailed(&saves.join("excluded"), &saves)
        .err()
        .expect("save exclusion");
    assert_eq!(failure.identity_error(), IdentityError::PrivateStorage);
    let diagnostic = failure
        .diagnostic()
        .expect("same-call save exclusion evidence");
    assert_eq!(diagnostic.operation(), PrivateOperation::VaultOpen);
    assert_eq!(diagnostic.stage(), PrivateStage::RootRelation);
    assert_eq!(diagnostic.cause(), PrivateCause::Refused);
}
