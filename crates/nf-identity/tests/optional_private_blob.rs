#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::private_storage::PrivateVault;
use std::fs;

#[test]
fn absent_blob_is_distinct_from_a_present_record() {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    assert_eq!(
        vault.read_optional_private_blob("receipt-r0-g0").unwrap(),
        None
    );
}
fn fixture() -> (temp_dir::Disposable, PrivateVault) {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    (root, vault)
}

#[test]
fn present_valid_and_damaged_blobs_never_become_absent() {
    let (root, vault) = fixture();
    vault.create_private_blob("valid", &[4, 5]).unwrap();
    assert_eq!(
        vault.read_optional_private_blob("valid").unwrap(),
        Some(vec![4, 5])
    );
    let valid = fs::read(root.join("private/blob-valid")).unwrap();
    let mut bad_magic = valid.clone();
    bad_magic[0] ^= 1;
    let mut bad_count = valid.clone();
    bad_count[10..14].copy_from_slice(&4097u32.to_le_bytes());
    let mut bad_checksum = valid.clone();
    *bad_checksum.last_mut().unwrap() ^= 1;
    for (index, damaged) in [
        vec![0; 45],
        vec![0; 4143],
        bad_magic,
        bad_count,
        bad_checksum,
        valid[..valid.len() - 1].to_vec(),
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("damaged-{index}");
        vault.create_private_blob(&name, &[4, 5]).unwrap();
        fs::write(root.join(format!("private/blob-{name}")), damaged).unwrap();
        assert_eq!(
            vault.read_optional_private_blob(&name),
            Err(nf_identity::model::IdentityError::MissingLocalState)
        );
    }
}

#[test]
fn invalid_names_and_nonregular_entries_are_refused() {
    let (root, vault) = fixture();
    for name in ["", "../identity-key-v1", "a/b", "a\\b", "a b"] {
        assert_eq!(
            vault.read_optional_private_blob(name),
            Err(nf_identity::model::IdentityError::PrivateStorage)
        );
    }
    fs::create_dir(root.join("private/blob-directory")).unwrap();
    assert_eq!(
        vault.read_optional_private_blob("directory"),
        Err(nf_identity::model::IdentityError::PrivateStorage)
    );
}

#[test]
fn a_missing_or_regular_file_root_cannot_be_an_empty_book() {
    use nf_identity::model::IdentityError;
    let (root, vault) = fixture();
    fs::remove_dir(root.join("private")).unwrap();
    assert_eq!(
        vault.read_optional_private_blob("absent"),
        Err(IdentityError::PrivateStorage)
    );
    let (root, vault) = fixture();
    vault.create_private_blob("replacement", &[]).unwrap();
    fs::rename(root.join("private/blob-replacement"), root.join("held")).unwrap();
    fs::remove_dir(root.join("private")).unwrap();
    fs::rename(root.join("held"), root.join("private")).unwrap();
    // The moved owner-private file passes the old root access check; it is not a vault directory.
    assert_eq!(
        vault.read_private_blob("absent"),
        Err(IdentityError::MissingLocalState)
    );
    assert_eq!(
        vault.read_optional_private_blob("absent"),
        Err(IdentityError::PrivateStorage)
    );
}

#[cfg(unix)]
#[test]
fn dangling_links_and_group_readable_entries_are_not_absence() {
    use nf_identity::model::IdentityError;
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (root, vault) = fixture();
    symlink(root.join("missing-target"), root.join("private/blob-link")).unwrap();
    assert_eq!(
        vault.read_optional_private_blob("link"),
        Err(IdentityError::PrivateStorage)
    );
    vault.create_private_blob("relaxed", &[4]).unwrap();
    fs::set_permissions(
        root.join("private/blob-relaxed"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    assert_eq!(
        vault.read_optional_private_blob("relaxed"),
        Err(IdentityError::PrivateStorage)
    );
}
