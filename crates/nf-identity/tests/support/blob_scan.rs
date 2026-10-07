#[path = "temp_dir.rs"]
mod temp_dir;
use nf_identity::private_storage::PrivateVault;
use std::fs;
pub use temp_dir::Disposable;
pub fn fixture() -> (Disposable, PrivateVault) {
    let root = temp_dir::disposable();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("private"), &saves).unwrap();
    (root, vault)
}
pub fn names() -> Vec<String> {
    (0..8)
        .flat_map(|slot| (0..8).map(move |generation| format!("receipt-r{slot}-g{generation}")))
        .collect()
}
