#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::private_storage::PrivateVault;
use std::fs;
#[test]
fn absence_does_not_end_the_complete_reserved_inventory() {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    vault.create_private_blob("receipt-r7-g7", &[7]).unwrap();
    let names: Vec<_> = (0..8)
        .flat_map(|slot| (0..8).map(move |generation| format!("receipt-r{slot}-g{generation}")))
        .collect();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    let mut visited = 0;
    vault
        .scan_optional_private_blobs_detailed(&names, |index, payload| {
            assert_eq!(index, visited);
            if index == 63 {
                assert_eq!(payload, Some([7].as_slice()));
            } else {
                assert_eq!(payload, None);
            }
            visited += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(visited, 64);
}
