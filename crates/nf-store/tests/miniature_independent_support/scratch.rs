pub struct Scratch {
    directory: std::path::PathBuf,
    root: std::path::PathBuf,
}
impl Scratch {
    pub fn new() -> Self {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
        std::fs::create_dir_all(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let mut nonce = [0u8; 8];
        getrandom::fill(&mut nonce).unwrap();
        let name = format!(
            "independent-{}-{:016x}",
            std::process::id(),
            u64::from_le_bytes(nonce)
        );
        let directory = root.join(name);
        std::fs::create_dir(&directory).unwrap();
        let directory = std::fs::canonicalize(directory).unwrap();
        assert!(directory.starts_with(&root) && directory != root);
        Self { directory, root }
    }
    pub fn database(&self) -> std::path::PathBuf {
        self.directory.join("store.sqlite")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(target) = std::fs::canonicalize(&self.directory)
            && target == self.directory
            && target != self.root
            && target.starts_with(&self.root)
        {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}
