use nf_identity::private_storage::PrivateVault;
use std::path::PathBuf;
pub struct Scratch {
    pub root: PathBuf,
}
impl Scratch {
    pub fn new() -> Self {
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).unwrap();
        let suffix = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let root = std::env::temp_dir().join(format!("nf-receipt-effects-{suffix}"));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("saves")).unwrap();
        Self { root }
    }
    pub fn vault(&self, name: &str) -> PrivateVault {
        PrivateVault::create(&self.root.join(name), &self.root.join("saves")).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
