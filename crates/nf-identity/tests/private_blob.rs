#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::{model::IdentityError, private_storage::PrivateVault};
use std::fs;
#[test]
fn private_rendezvous_blob_is_immutable_bounded_and_cannot_address_identity_keys() {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    vault
        .create_private_blob("ipc_session-1", &[9; 32])
        .unwrap();
    assert_eq!(
        vault.read_private_blob("ipc_session-1").unwrap(),
        vec![9; 32]
    );
    assert_eq!(
        vault.create_private_blob("ipc_session-1", &[8; 32]),
        Err(IdentityError::PrivateStorage)
    );
    for name in [
        "../identity-key-v1",
        "\\absolute",
        "a/b",
        "",
        ".",
        "ä",
        "a b",
    ] {
        assert_eq!(
            vault.create_private_blob(name, &[0]),
            Err(IdentityError::PrivateStorage)
        );
    }
    assert_eq!(
        vault.create_private_blob("oversized", &vec![0; 4097]),
        Err(IdentityError::Limit)
    );
    vault.remove_private_blob("ipc_session-1").unwrap();
    assert_eq!(
        vault.read_private_blob("ipc_session-1").err(),
        Some(IdentityError::MissingLocalState)
    );
}
