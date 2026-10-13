//! Public synthetic backup evidence is retained on both success and failure.
//! Keys/signers live only in the maintained RAM fixture; no vault or save is used.
use super::miniature_support::{Fixture, claim};
use nf_store::miniature::{
    MiniatureKnownFrontiers, MiniatureReader, MiniatureSnapshot, MiniatureStore,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct RetainedBackup {
    pub root: PathBuf,
    pub source: PathBuf,
    pub backup: PathBuf,
    pub known: MiniatureKnownFrontiers,
    snapshot: MiniatureSnapshot,
    state: Vec<u8>,
    started: Instant,
}

impl RetainedBackup {
    pub fn new(case: &str) -> Self {
        let started = Instant::now();
        let parent = PathBuf::from(
            std::env::var_os("NF_ISOLATED_RESTORE_REFUSAL_EVIDENCE_PARENT")
                .expect("ROOT_OWNED_RESTORE_REFUSAL_PARENT_REQUIRED"),
        );
        assert!(parent.is_absolute(), "caller must supply an absolute owned parent");
        let metadata = fs::symlink_metadata(&parent).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_eq!(metadata.file_attributes() & 0x400, 0);
        }
        let parent = parent.canonicalize().unwrap();
        let nonce = nf_identity::keys::random_id().expect("fresh public evidence nonce");
        let nonce: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
        let root = parent.join(format!(
            "{case}-{}-{}-{nonce}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("retain prior evidence rather than reuse a case directory");
        let source = root.join("original.sqlite");
        let backup = root.join("backup.sqlite");
        let mut fixture = Fixture::new();
        let mut writer = MiniatureStore::create(
            &source,
            fixture.spec,
            &fixture.bytes(),
            fixture.policy.clone(),
            &mut fixture.signer,
        )
        .unwrap();
        claim(&mut writer, &mut fixture);
        let known = writer.known_frontiers().unwrap();
        assert_eq!(known.storage.store_revision, 1);
        assert_eq!(known.storage.event_sequence.0, 0);
        assert_eq!(known.storage.membership_revision, Some(2));
        assert_eq!(known.minimum_authority_term.0, 1);
        let snapshot = writer.snapshot().unwrap();
        let state = writer.snapshot_bytes().unwrap();
        assert_eq!(writer.backup_to(&backup).unwrap(), known);
        drop(writer);
        // No deleting destructor: the caller retains this exact directory.
        Self {
            root,
            source,
            backup,
            known,
            snapshot,
            state,
            started,
        }
    }

    pub fn assert_original(&self) {
        let reader = MiniatureReader::open_existing(&self.source, self.known).unwrap();
        assert!(reader.is_read_only().unwrap());
        assert_eq!(reader.snapshot(), &self.snapshot);
        assert_eq!(reader.snapshot_bytes().unwrap(), self.state);
        drop(reader);
        assert!(self.started.elapsed() < Duration::from_secs(20));
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Entry {
    Directory,
    File(Vec<u8>),
}

/// An independent byte-and-name observation of the entire tiny owned case tree.
pub fn tree(root: &Path) -> BTreeMap<PathBuf, Entry> {
    let mut result = BTreeMap::new();
    let mut remaining = vec![root.to_path_buf()];
    let mut total = 0usize;
    while let Some(path) = remaining.pop() {
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        if metadata.is_dir() {
            result.insert(relative, Entry::Directory);
            for entry in fs::read_dir(&path).unwrap() {
                remaining.push(entry.unwrap().path());
                assert!(remaining.len() <= 64);
            }
        } else {
            assert!(metadata.is_file() && metadata.len() <= 1_048_576);
            let bytes = fs::read(&path).unwrap();
            total = total.checked_add(bytes.len()).unwrap();
            assert!(total <= 16_777_216);
            result.insert(relative, Entry::File(bytes));
        }
        assert!(result.len() <= 64);
    }
    result
}
