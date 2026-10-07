#[path = "support/blob_scan.rs"]
mod support;
use nf_identity::model::IdentityError;
use std::fs;
#[test]
fn present_file_bounds_refuse_short_and_maximum_plus_one_records_before_visit() {
    let (root, vault) = support::fixture();
    let names = support::names();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    vault.create_private_blob(names[0], &[7]).unwrap();
    for size in [45, 4143] {
        fs::write(root.join("private/blob-receipt-r0-g0"), vec![7; size]).unwrap();
        assert_eq!(
            vault.scan_optional_private_blobs(&names, |_, _| panic!("invalid size admitted")),
            Err(IdentityError::MissingLocalState)
        );
    }
}
