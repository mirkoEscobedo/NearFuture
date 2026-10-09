#![cfg(windows)]
#[path = "support/getter_contract/process.rs"]
mod process;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::{model::IdentityError, private_storage::PrivateVault};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

struct Fixture {
    _scratch: temp_dir::Disposable,
    root: PathBuf,
    old: PathBuf,
    old_bytes: Vec<u8>,
    vault: PrivateVault,
}
impl Fixture {
    fn new() -> Self {
        let scratch = temp_dir::disposable();
        let root = scratch.join("private");
        let saves = scratch.join("saves");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&saves).unwrap();
        let original = support("getter_contract/original-private-acl.ps1");
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(&original).unwrap())),
            "d83b47582ed20336ac79ff674adfb40846bed3ccec4416aad7ef9107bf3ad0aa"
        );
        initialize(&original, &root, "root");
        let vault = PrivateVault::open(&root, &saves).expect("genuine original protected setup");
        vault.create_private_blob("receipt-r0-g0", &[3, 7]).unwrap();
        let old = root.join("blob-receipt-r0-g0");
        assert_eq!(
            vault
                .read_private_blob("receipt-r0-g0")
                .expect("fresh protected blob ACL and record verification"),
            [3, 7]
        );
        let old_bytes = fs::read(&old).unwrap();
        Self {
            _scratch: scratch,
            root,
            old,
            old_bytes,
            vault,
        }
    }
    fn old_unchanged(&self) {
        assert_eq!(fs::read(&self.old).unwrap(), self.old_bytes);
    }
    fn candidate(&self) -> PathBuf {
        self.root.join("blob-receipt-r0-g1")
    }
}
fn support(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support")
        .join(name)
}
fn initialize(script: &Path, path: &Path, stage: &'static str) {
    assert_eq!(
        process::run(
            script,
            &[
                OsStr::new("-PrivatePath"),
                path.as_os_str(),
                OsStr::new("-Initialize")
            ]
        ),
        0,
        "original initializer setup failed: stage={stage}, exists={}, file={}, directory={}",
        path.exists(),
        path.is_file(),
        path.is_dir()
    );
}
fn relax(path: &Path) {
    assert_eq!(
        process::run(
            &support("acl_session/relax-acl.ps1"),
            &[OsStr::new("-PrivatePath"), path.as_os_str()]
        ),
        0,
        "genuine ACL mutation setup failed"
    );
}

#[test]
fn postread_root_acl_change_refuses_completion_after_provisional_visit() {
    let fixture = Fixture::new();
    let mut scope = fixture.vault.protected_blob_scope().unwrap();
    let mut seen = Vec::new();
    let result = scope.scan_optional_private_blobs(&["receipt-r0-g0"], |index, bytes| {
        seen.push((index, bytes.map(Vec::from)));
        relax(&fixture.root);
        Ok(())
    });
    assert_eq!(seen, [(0, Some(vec![3, 7]))]);
    assert_eq!(result, Err(IdentityError::PrivateStorage));
    assert_eq!(scope.finish_readonly(), Err(IdentityError::PrivateStorage));
    assert!(!fixture.candidate().exists());
    fixture.old_unchanged();
}

#[test]
fn precreate_root_acl_change_refuses_before_candidate_open() {
    let fixture = Fixture::new();
    let mut scope = fixture.vault.protected_blob_scope().unwrap();
    scope
        .scan_optional_private_blobs(&["receipt-r0-g0"], |_, _| Ok(()))
        .unwrap();
    relax(&fixture.root);
    assert_eq!(
        scope.create_private_blob("receipt-r0-g1", &[11, 13]),
        Err(IdentityError::PrivateStorage)
    );
    assert!(!fixture.candidate().exists());
    assert_eq!(scope.finish_append(), Err(IdentityError::PrivateStorage));
    fixture.old_unchanged();
}

#[test]
fn reread_existing_file_acl_change_refuses_catalog_without_rollback_claim() {
    let fixture = Fixture::new();
    let mut scope = fixture.vault.protected_blob_scope().unwrap();
    scope
        .scan_optional_private_blobs(&["receipt-r0-g0", "receipt-r0-g1"], |_, _| Ok(()))
        .unwrap();
    relax(&fixture.old);
    scope
        .create_private_blob("receipt-r0-g1", &[11, 13])
        .unwrap();
    let candidate = fs::read(fixture.candidate()).unwrap();
    let mut seen = Vec::new();
    assert_eq!(
        scope.scan_optional_private_blobs(&["receipt-r0-g0", "receipt-r0-g1"], |index, _| {
            seen.push(index);
            Ok(())
        }),
        Err(IdentityError::PrivateStorage)
    );
    assert!(seen.is_empty(), "V5 must reject before reread visitors");
    assert_eq!(scope.finish_append(), Err(IdentityError::PrivateStorage));
    assert_eq!(fs::read(fixture.candidate()).unwrap(), candidate);
    fixture.old_unchanged();
}

struct Junction(PathBuf);
impl Drop for Junction {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}
#[test]
fn real_reparse_candidate_is_refused_and_external_target_is_unchanged() {
    let fixture = Fixture::new();
    let external = fixture._scratch.join("external");
    fs::create_dir(&external).unwrap();
    let marker = external.join("public-marker");
    fs::write(&marker, b"external public fixture").unwrap();
    let before = fs::read(&marker).unwrap();
    let candidate = fixture.candidate();
    let link = Junction(candidate.clone());
    assert_eq!(
        process::run(
            &support("acl_session/junction.ps1"),
            &[
                OsStr::new("-PrivatePath"),
                candidate.as_os_str(),
                OsStr::new("-Target"),
                external.as_os_str()
            ]
        ),
        0,
        "real Junction setup failed"
    );
    assert_ne!(
        fs::symlink_metadata(&candidate).unwrap().file_attributes() & 0x400,
        0
    );
    let mut scope = fixture.vault.protected_blob_scope().unwrap();
    assert_eq!(
        scope.scan_optional_private_blobs(&["receipt-r0-g0", "receipt-r0-g1"], |_, _| Ok(())),
        Err(IdentityError::PrivateStorage)
    );
    drop(scope);
    let mut scope = fixture.vault.protected_blob_scope().unwrap();
    scope
        .scan_optional_private_blobs(&["receipt-r0-g0"], |_, _| Ok(()))
        .unwrap();
    assert_eq!(
        scope.create_private_blob("receipt-r0-g1", &[11, 13]),
        Err(IdentityError::PrivateStorage)
    );
    drop(scope);
    assert_eq!(fs::read(&marker).unwrap(), before);
    assert_eq!(fs::read_dir(&external).unwrap().count(), 1);
    fixture.old_unchanged();
    drop(link);
    assert!(!candidate.exists());
    assert_eq!(fs::read(&marker).unwrap(), before);
}
