//! Caller-created custody and one fixed deadline; synchronous I/O has bounded bytes.
use super::records::hash;
use nf_identity::keys::random_id;
use nf_store::managed::FilePin;
use std::{
    fs::{File, Metadata, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
pub const MAX_WORLD_FILE: u64 = 8 * 1024 * 1024;
pub const MAX_BINARY_FILE: u64 = 64 * 1024 * 1024;
pub const MAX_BINARY_TREE: u64 = 2 * MAX_BINARY_FILE;
pub const MAX_WORLD_TREE: u64 = 64 * 1024 * 1024;
pub struct Case {
    root: PathBuf,
    binaries: PathBuf,
    started: Instant,
    cutoff: u64,
    start: u64,
}
fn ordinary(meta: &Metadata) {
    assert!(!meta.file_type().is_symlink());
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(meta.file_attributes() & 0x400, 0, "reparse point refused");
    }
}
fn parent(name: &str) -> PathBuf {
    let path =
        PathBuf::from(std::env::var_os(name).expect("caller must create owned custody parent"));
    let meta = std::fs::symlink_metadata(&path).unwrap();
    ordinary(&meta);
    assert!(meta.is_dir(), "caller parent must be a directory");
    path.canonicalize().unwrap()
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
pub fn read_with_cap(path: &Path, cap: u64) -> Vec<u8> {
    ordinary(&std::fs::symlink_metadata(path).unwrap());
    let mut file = File::open(path).unwrap();
    let before = file.metadata().unwrap();
    assert!(before.is_file());
    assert!(before.len() <= cap);
    let mut bytes = Vec::new();
    (&mut file)
        .take(cap.checked_add(1).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(
        u64::try_from(bytes.len()).unwrap() <= cap,
        "hard read cap exceeded"
    );
    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        before.len(),
        "EOF/length mismatch"
    );
    let after = file.metadata().unwrap();
    assert_eq!(after.len(), before.len(), "handle length changed");
    assert_eq!(
        after.modified().unwrap(),
        before.modified().unwrap(),
        "handle changed"
    );
    bytes
}
fn tree_bytes(path: &Path, file_cap: u64, tree_cap: u64) -> u64 {
    let mut total = 0u64;
    let mut count = 0usize;
    for entry in std::fs::read_dir(path).unwrap() {
        count += 1;
        assert!(count <= 16, "custody tree entry cap");
        let entry = entry.unwrap();
        let meta = std::fs::symlink_metadata(entry.path()).unwrap();
        ordinary(&meta);
        assert!(meta.is_file(), "no nested/unbounded tree");
        assert!(meta.len() <= file_cap);
        total = total.checked_add(meta.len()).unwrap();
        assert!(total <= tree_cap);
    }
    total
}
pub fn bounded_read(path: &Path) -> Vec<u8> {
    read_with_cap(path, MAX_WORLD_FILE)
}
impl Case {
    pub fn new() -> Self {
        // The single clock starts before every environment/filesystem/fixture effect.
        let started = Instant::now();
        let start = now_ms();
        let cutoff = start.checked_add(15_000).unwrap();
        let world_parent = parent("NF_MANAGED_UPGRADE_EVIDENCE_ROOT");
        let binary_parent = parent("NF_MANAGED_UPGRADE_BINARY_ROOT");
        assert!(!world_parent.starts_with(&binary_parent));
        assert!(!binary_parent.starts_with(&world_parent));
        let nonce = random_id()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let root = world_parent.join(&nonce);
        let binaries = binary_parent.join(&nonce);
        assert!(started.elapsed().as_millis() < 15_000);
        std::fs::create_dir(&root).unwrap();
        assert!(started.elapsed().as_millis() < 15_000);
        std::fs::create_dir(&binaries).unwrap();
        assert!(started.elapsed().as_millis() < 15_000);
        let value = Self {
            root,
            binaries,
            started,
            cutoff,
            start,
        };
        value.note("scope.txt", b"Caller-created separate custody; preserve partial files. No private seeds. Binary evidence is a running test executable, operational compatibility NOT QUALIFIED.\n");
        value
    }
    pub fn source(&self) -> PathBuf {
        self.root.join("source.sqlite")
    }
    pub fn target(&self) -> PathBuf {
        self.root.join("active.sqlite")
    }
    pub fn catalog(&self) -> PathBuf {
        self.root.join("owner-catalog.sqlite")
    }
    pub fn retained_source(&self) -> PathBuf {
        self.root.join("retained-source.sqlite")
    }
    pub fn old_binary(&self) -> PathBuf {
        self.binaries.join("retained-test-executable.bin")
    }
    pub fn new_binary(&self) -> PathBuf {
        self.binaries.join("new-test-executable.bin")
    }
    pub fn start(&self) -> u64 {
        self.start
    }
    pub fn check(&self) {
        assert!(
            self.started.elapsed().as_millis() < 15_000,
            "single caller cutoff exhausted"
        );
    }
    pub fn cutoff(&self) -> u64 {
        self.check();
        self.cutoff
    }
    fn note(&self, name: &str, bytes: &[u8]) {
        self.check();
        assert!(bytes.len() <= 65_536);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(name))
            .unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
        drop(file);
        self.check(); // Even final fsync cannot silently extend the deadline.
    }
    pub fn binary_pin(&self, path: PathBuf) -> FilePin {
        self.check();
        assert!(path.starts_with(&self.binaries));
        let bytes = read_with_cap(&std::env::current_exe().unwrap(), MAX_BINARY_FILE);
        self.check();
        assert!(bytes.starts_with(b"MZ") || bytes.starts_with(b"\x7fELF"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(&bytes).unwrap();
        file.sync_all().unwrap();
        drop(file);
        self.check();
        assert_eq!(read_with_cap(&path, MAX_BINARY_FILE), bytes);
        self.check();
        FilePin::new(
            path,
            bytes.len().try_into().unwrap(),
            hash(&bytes),
            "Running test executable; operational old/new node compatibility NOT QUALIFIED",
        )
        .unwrap()
    }
    pub fn finish(self) {
        self.check();
        assert!(tree_bytes(&self.binaries, MAX_BINARY_FILE, MAX_BINARY_TREE) <= MAX_BINARY_TREE);
        assert!(tree_bytes(&self.root, MAX_WORLD_FILE, MAX_WORLD_TREE) <= MAX_WORLD_TREE);
        self.note("complete.txt", b"Library assertions completed. Actual operational executable compatibility NOT QUALIFIED. Byte-capped synchronous I/O cannot preempt an OS call; all returning effects must meet the unchanged single15s cut.\n");
        self.check();
        assert!(tree_bytes(&self.root, MAX_WORLD_FILE, MAX_WORLD_TREE) <= MAX_WORLD_TREE);
        self.check();
    }
}
// No Drop deletion/retry/poll/process launch. Separate roots belong to the caller.
