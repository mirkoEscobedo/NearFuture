use super::{hash, permissions};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_FILES: usize = 64;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_TEXT: usize = 1024 * 1024;
fn check_clock(started: &Instant) {
    assert!(
        started.elapsed() <= Duration::from_secs(20),
        "fixed absolute case deadline"
    );
}
fn with_clock<T>(started: &Instant, call: impl FnOnce() -> T) -> T {
    check_clock(started);
    let result = call();
    check_clock(started);
    result
}
#[derive(Clone, Eq, PartialEq)]
pub struct Entry {
    directory: bool,
    permissions: permissions::Permissions,
    bytes: Vec<u8>,
}
pub type Tree = BTreeMap<PathBuf, Entry>;
pub struct Case {
    started: Instant,
    root: PathBuf,
}
impl Case {
    pub fn new() -> Self {
        // This single clock precedes environment, randomness and every fixture effect.
        let started = Instant::now();
        let parent = PathBuf::from(
            with_clock(&started, || {
                std::env::var_os("NF_STRATEGIC_READER_EVIDENCE_ROOT")
            })
            .expect("caller-created NF_STRATEGIC_READER_EVIDENCE_ROOT required"),
        );
        with_clock(&started, || permissions::directory(&parent));
        let parent = with_clock(&started, || parent.canonicalize()).unwrap();
        with_clock(&started, || permissions::directory(&parent));
        let random = with_clock(&started, || nf_identity::keys::random_id()).unwrap();
        let nonce: String = random.iter().map(|v| format!("{v:02x}")).collect();
        let root = parent.join(format!("strategic-reader-{nonce}"));
        with_clock(&started, || fs::create_dir(&root)).unwrap();
        let case = Self { started, root };
        case.check();
        for dir in ["inputs", "excluded", "control", "public"] {
            case.check();
            case.guard(|| fs::create_dir(case.root.join(dir))).unwrap();
            case.check();
        }
        for (name, bytes) in [
            (
                "private-chat-marker.txt",
                b"NF_FAKE_PRIVATE_CHAT_DO_NOT_EXPORT".as_slice(),
            ),
            (
                "key-material-marker.txt",
                b"NF_FAKE_KEY_MATERIAL_DO_NOT_EXPORT".as_slice(),
            ),
            (
                "save-content-marker.txt",
                b"NF_FAKE_SAVE_CONTENT_DO_NOT_EXPORT".as_slice(),
            ),
        ] {
            case.write_new(&case.root.join("excluded").join(name), bytes, MAX_TEXT);
        }
        case
    }
    pub fn check(&self) {
        check_clock(&self.started);
    }
    pub fn guard<T>(&self, call: impl FnOnce() -> T) -> T {
        with_clock(&self.started, call)
    }
    pub fn original(&self) -> PathBuf {
        self.root.join("inputs/original.sqlite")
    }
    pub fn backup(&self) -> PathBuf {
        self.root.join("inputs/old.sqlite")
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn make_read_only(&self, path: &Path) {
        self.check();
        assert!(
            path == self.original() || path == self.backup(),
            "only fresh fixture databases"
        );
        let metadata = self.guard(|| fs::symlink_metadata(path)).unwrap();
        permissions::plain(&metadata);
        let mut value = metadata.permissions();
        value.set_readonly(true);
        self.guard(|| fs::set_permissions(path, value)).unwrap();
        assert!(
            self.guard(|| fs::symlink_metadata(path))
                .unwrap()
                .permissions()
                .readonly()
        );
        self.check();
    }
    pub fn read(&self, path: &Path, cap: usize) -> Vec<u8> {
        self.check();
        let observed = self.guard(|| fs::symlink_metadata(path)).unwrap();
        permissions::plain(&observed);
        assert!(observed.is_file());
        let file = self.guard(|| File::open(path)).unwrap();
        let before = self.guard(|| file.metadata()).unwrap();
        assert!(before.len() <= cap as u64);
        let before_modified = self.guard(|| before.modified()).unwrap();
        let bytes = {
            let mut reader = (&file).take(cap as u64 + 1);
            let mut bytes = Vec::new();
            let mut buffer = [0; 32768];
            loop {
                let count = self.guard(|| reader.read(&mut buffer)).unwrap();
                if count == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..count]);
            }
            assert!(bytes.len() <= cap && bytes.len() as u64 == before.len());
            let mut eof = [0; 1];
            assert_eq!(self.guard(|| reader.read(&mut eof)).unwrap(), 0);
            let after = self.guard(|| file.metadata()).unwrap();
            assert_eq!(before.len(), after.len());
            assert_eq!(before_modified, self.guard(|| after.modified()).unwrap());
            assert!(permissions::observe(&before) == permissions::observe(&after));
            assert_eq!(observed.len(), before.len());
            bytes
        };
        self.guard(|| drop(file));
        self.check();
        bytes
    }
    pub fn write_new(&self, path: &Path, bytes: &[u8], cap: usize) {
        self.check();
        assert!(path.starts_with(&self.root) && bytes.len() <= cap);
        let mut file = self
            .guard(|| OpenOptions::new().write(true).create_new(true).open(path))
            .unwrap();
        for chunk in bytes.chunks(32768) {
            self.check();
            let mut offset = 0;
            while offset < chunk.len() {
                self.check();
                let written = self.guard(|| file.write(&chunk[offset..])).unwrap();
                assert!(written > 0, "zero progress fixture write");
                offset += written;
            }
        }
        self.check();
        self.guard(|| file.sync_all()).unwrap();
        self.check();
        self.guard(|| drop(file));
        assert!(self.read(path, cap) == bytes);
        self.check();
    }
    pub fn retain_baselines(&self, old: &[u8], new: &[u8], member0: &[u8], member1: &[u8]) {
        for (name, bytes) in [
            ("old-canonical.bin", old),
            ("new-canonical.bin", new),
            ("old-public-membership.bin", member0),
            ("new-public-membership.bin", member1),
        ] {
            self.write_new(&self.root.join("control").join(name), bytes, MAX_TEXT);
        }
        for (name, path) in [
            ("original.before.bin", self.original()),
            ("backup.before.bin", self.backup()),
        ] {
            let bytes = self.read(&path, MAX_BYTES);
            self.write_new(&self.root.join("control").join(name), &bytes, MAX_BYTES);
        }
        self.tree();
    }
    pub fn tree(&self) -> Tree {
        let mut result = Tree::new();
        let mut total = 0;
        self.walk(&self.root, 0, &mut total, &mut result);
        assert!(result.len() <= MAX_FILES && total <= MAX_BYTES);
        self.check();
        result
    }
    fn walk(&self, path: &Path, depth: usize, total: &mut usize, result: &mut Tree) {
        self.check();
        assert!(depth <= 2);
        let metadata = self.guard(|| fs::symlink_metadata(path)).unwrap();
        permissions::plain(&metadata);
        let bytes = if metadata.is_file() {
            self.read(path, MAX_BYTES.saturating_sub(*total))
        } else {
            Vec::new()
        };
        *total = (*total).checked_add(bytes.len()).unwrap();
        assert!(*total <= MAX_BYTES && result.len() < MAX_FILES);
        let key = path.strip_prefix(&self.root).unwrap().to_path_buf();
        assert!(
            result
                .insert(
                    key,
                    Entry {
                        directory: metadata.is_dir(),
                        permissions: permissions::observe(&metadata),
                        bytes,
                    }
                )
                .is_none()
        );
        if metadata.is_dir() {
            let mut children = Vec::new();
            let mut entries = self.guard(|| fs::read_dir(path)).unwrap();
            for _ in 0..MAX_FILES + 1 {
                let Some(item) = self.guard(|| entries.next()) else {
                    break;
                };
                children.push(item.unwrap().path());
            }
            self.guard(|| drop(entries));
            assert!(children.len() <= MAX_FILES);
            children.sort();
            for child in children {
                self.walk(&child, depth + 1, total, result);
            }
        }
        self.check();
    }
    pub fn finish(&self, projection: &str, before: &Tree) {
        assert!(self.tree() == *before);
        self.write_new(
            &self.root.join("public/observation.json"),
            projection.as_bytes(),
            MAX_TEXT,
        );
        self.write_new(&self.root.join("public/capture-receipt.txt"),
            format!("One library case; no children; fixed20s deadline; <=64 entries/8MiB total. Original, backup, canonical/public membership and selected tree compared unchanged around admission/observation, each independent minimum refusal, and reopen sequence. {} Synchronous I/O is checked before/after returning calls and cannot be forcibly preempted; no deadline extension. No full export/current-owner/native/whole-issue qualification.\n",
                permissions::OBSERVATION_STRENGTH).as_bytes(), MAX_TEXT);
        let tree = self.tree();
        assert_eq!(tree.len(), before.len() + 2);
        for (key, value) in before {
            assert!(
                tree.get(key) == Some(value),
                "final capture altered preexisting fixture"
            );
        }
        let digest = hash(projection.as_bytes());
        assert_eq!(digest, super::projection::EXPECTED_SHA256);
        self.check();
    }
}
// No Drop: all successful, refused and partial fixture files remain caller-owned.
