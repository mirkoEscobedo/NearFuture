#[path = "support/blob_scan.rs"]
mod support;
use nf_identity::model::IdentityError;
use std::fs;
#[test]
fn invalid_or_colliding_input_cannot_admit_any_callback() {
    let (_root, vault) = support::fixture();
    let over = vec!["record"; 65];
    let mut callbacks = 0;
    for names in [vec![], over] {
        assert_eq!(
            vault.scan_optional_private_blobs(&names, |_, _| {
                callbacks += 1;
                Ok(())
            }),
            Err(IdentityError::Limit)
        );
    }
    for names in [
        vec!["a", "a"],
        vec!["a", "A"],
        vec!["../identity-key-v1"],
        vec!["a/b"],
        vec![""],
    ] {
        assert_eq!(
            vault.scan_optional_private_blobs(&names, |_, _| {
                callbacks += 1;
                Ok(())
            }),
            Err(IdentityError::PrivateStorage)
        );
    }
    assert_eq!(callbacks, 0);
}
#[test]
fn corrupt_tail_after_absent_entries_refuses_the_complete_pass() {
    let (root, vault) = support::fixture();
    vault.create_private_blob("receipt-r7-g7", &[7]).unwrap();
    fs::write(root.join("private/blob-receipt-r7-g7"), [0u8; 47]).unwrap();
    let names = support::names();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    let mut callbacks = 0;
    assert_eq!(
        vault.scan_optional_private_blobs(&names, |_, _| {
            callbacks += 1;
            Ok(())
        }),
        Err(IdentityError::MissingLocalState)
    );
    assert_eq!(callbacks, 63); // Provisional observations are discarded on Err by the book owner.
}
#[test]
fn visitor_refusal_stops_without_a_successful_inventory() {
    let (_root, vault) = support::fixture();
    let mut callbacks = 0;
    assert_eq!(
        vault.scan_optional_private_blobs(&["first", "second", "third"], |index, _| {
            callbacks += 1;
            if index == 1 {
                Err(IdentityError::Limit)
            } else {
                Ok(())
            }
        }),
        Err(IdentityError::Limit)
    );
    assert_eq!(callbacks, 2);
}
#[test]
fn invalid_root_or_nonregular_present_entry_is_never_absence() {
    let (root, vault) = support::fixture();
    fs::create_dir(root.join("private/blob-directory")).unwrap();
    assert_eq!(
        vault.scan_optional_private_blobs(&["directory"], |_, _| panic!("invalid entry admitted")),
        Err(IdentityError::PrivateStorage)
    );
    fs::remove_dir(root.join("private/blob-directory")).unwrap();
    fs::remove_dir(root.join("private")).unwrap();
    assert_eq!(
        vault.scan_optional_private_blobs(&["absent"], |_, _| panic!("invalid root admitted")),
        Err(IdentityError::PrivateStorage)
    );
}
#[test]
fn a_present_entry_disappearing_after_admission_is_an_error() {
    let (root, vault) = support::fixture();
    vault.create_private_blob("last", &[7]).unwrap();
    assert_eq!(
        vault.scan_optional_private_blobs(&["absent", "last"], |index, _| {
            // Deliberate sequential filesystem fault injection, not a permitted consumer action.
            if index == 0 {
                fs::remove_file(root.join("private/blob-last")).unwrap();
            }
            Ok(())
        }),
        Err(IdentityError::PrivateStorage)
    );
}
#[cfg(unix)]
#[test]
fn late_group_readable_mode_is_refused_before_callbacks() {
    use std::os::unix::fs::PermissionsExt;
    let (root, vault) = support::fixture();
    vault.create_private_blob("first", &[1]).unwrap();
    vault.create_private_blob("middle", &[2]).unwrap();
    vault.create_private_blob("last", &[3]).unwrap();
    fs::set_permissions(
        root.join("private/blob-middle"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    assert_eq!(
        vault.scan_optional_private_blobs(&["first", "middle", "last"], |_, _| panic!(
            "invalid mode admitted"
        )),
        Err(IdentityError::PrivateStorage)
    );
}
