use std::{
    collections::BTreeMap,
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const CAPTURE_BYTES: usize = 1_048_576;
const CASE_BYTES: usize = 8_388_608;
const CASE_ENTRIES: usize = 64;
const CALLER: &[u8] = b"{\"version\":1,\"kind\":\"owned-public-actual-producer-fixtures\",\"secretFiles\":false,\"children\":false,\"caseSeconds\":20,\"caseBytes\":8388608,\"caseEntries\":64}\n";
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub enum Stage {
    OwnedSetup,
    RamFixture,
    FixtureCapture,
    BaselineSnapshot,
    StartHost,
    RegisterSession,
    EvaluateAction,
    ReceiptAssertions,
    FirstObservation,
    SecondObservation,
    FirstProjection,
    SecondProjection,
    StopHost,
    TreePreservation,
    OutputCapture,
    LiteralAssertions,
    CompletionCheck,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Self::OwnedSetup => "owned_setup",
            Self::RamFixture => "ram_fixture",
            Self::FixtureCapture => "fixture_capture",
            Self::BaselineSnapshot => "baseline_snapshot",
            Self::StartHost => "start_host",
            Self::RegisterSession => "register_session",
            Self::EvaluateAction => "evaluate_action",
            Self::ReceiptAssertions => "receipt_assertions",
            Self::FirstObservation => "first_observation",
            Self::SecondObservation => "second_observation",
            Self::FirstProjection => "first_projection",
            Self::SecondProjection => "second_projection",
            Self::StopHost => "stop_host",
            Self::TreePreservation => "tree_preservation",
            Self::OutputCapture => "output_capture",
            Self::LiteralAssertions => "literal_assertions",
            Self::CompletionCheck => "completion_check",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Entry {
    bytes: Vec<u8>,
    metadata: String,
}

pub struct Evidence {
    root: PathBuf,
    started: Instant,
    entries: usize,
    bytes: usize,
    cleanup: bool,
    stage: Stage,
    start_utc_millis: u64,
    cutoff_utc_millis: u64,
}

fn clock(started: Instant, cleanup: bool) {
    let limit = if cleanup { 20 } else { 18 };
    assert!(
        started.elapsed() < Duration::from_secs(limit),
        "original case deadline exhausted"
    );
}

// Hold returned resources locally before the postcheck, so unwind releases them.
fn external<T>(started: Instant, cleanup: bool, operation: impl FnOnce() -> T) -> T {
    clock(started, cleanup);
    let result = operation();
    clock(started, cleanup);
    result
}

fn ordinary(metadata: &fs::Metadata) {
    assert!(
        !metadata.file_type().is_symlink(),
        "symlink evidence refused"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            metadata.file_attributes() & 0x400,
            0,
            "reparse evidence refused"
        );
    }
}

fn metadata_signature(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            metadata.dev(),
            metadata.ino(),
            metadata.mode(),
            metadata.nlink(),
            metadata.uid(),
            metadata.gid(),
            metadata.permissions().readonly(),
            metadata.len(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec()
        )
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        format!(
            "{}:{}:{}:{}:{}",
            metadata.file_attributes(),
            metadata.creation_time(),
            metadata.last_write_time(),
            metadata.file_size(),
            metadata.permissions().readonly()
        )
    }
    #[cfg(not(any(unix, windows)))]
    {
        format!(
            "{}:{:?}:{:?}",
            metadata.len(),
            metadata.permissions(),
            metadata.modified().unwrap()
        )
    }
}

fn admit(metadata: &fs::Metadata, allowance: usize) -> usize {
    ordinary(metadata);
    assert!(
        metadata.is_file() && metadata.len() <= CAPTURE_BYTES as u64,
        "bounded regular file required"
    );
    let length = usize::try_from(metadata.len()).unwrap();
    // Reserve one attempted EOF/overlength probe inside the remaining aggregate cap.
    assert!(
        length < allowance,
        "remaining read allowance must include one probe byte"
    );
    length
}

fn read_bounded(path: &Path, allowance: usize, started: Instant, cleanup: bool) -> (Entry, usize) {
    let before = external(started, cleanup, || fs::symlink_metadata(path)).unwrap();
    let expected = admit(&before, allowance);
    let signature = metadata_signature(&before);
    let mut file = external(started, cleanup, || File::open(path)).unwrap();
    let opened = external(started, cleanup, || file.metadata()).unwrap();
    assert_eq!(expected, admit(&opened, allowance));
    assert_eq!(
        signature,
        metadata_signature(&opened),
        "opened file changed"
    );
    let mut bytes = Vec::new();
    let mut block = [0; 32768];
    while bytes.len() < expected {
        let remaining = expected - bytes.len();
        let width = remaining.min(block.len());
        assert!(bytes.len() + width < allowance, "remaining chunk allowance");
        let count = external(started, cleanup, || file.read(&mut block[..width])).unwrap();
        assert!(count > 0, "file shrank during bounded read");
        assert!(bytes.len() + count <= expected && bytes.len() + count < allowance);
        bytes.extend_from_slice(&block[..count]);
    }
    let mut probe = [0; 1];
    assert!(bytes.len() < allowance, "EOF probe admission");
    let count = external(started, cleanup, || file.read(&mut probe)).unwrap();
    assert_eq!(count, 0, "file grew during bounded read");
    let after = external(started, cleanup, || file.metadata()).unwrap();
    assert_eq!(signature, metadata_signature(&after), "read file changed");
    // Release is unconditional, including unwind; this is not checked close evidence.
    external(started, cleanup, || drop(file));
    clock(started, cleanup);
    (
        Entry {
            bytes,
            metadata: signature,
        },
        expected + 1,
    )
}

impl Evidence {
    pub fn create(started: Instant, owned: &mut Option<Self>) {
        let start_utc_millis = external(started, false, || SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let start_utc_millis = u64::try_from(start_utc_millis).unwrap();
        let cutoff_utc_millis = start_utc_millis.checked_add(20_000).unwrap();
        let raw = external(started, false, || {
            env::var_os("NF_ACTUAL_PRODUCER_EVIDENCE_ROOT")
        })
        .map(PathBuf::from)
        .expect("caller evidence root required");
        assert!(raw.is_absolute(), "absolute existing caller root required");
        #[cfg(windows)]
        assert!(
            !raw.to_string_lossy().starts_with(r"\\"),
            "network or verbatim caller root refused"
        );
        assert!(
            !raw.components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        );
        let ancestors = raw.ancestors().collect::<Vec<_>>();
        assert!(ancestors.len() <= 32, "caller path depth limit");
        for ancestor in ancestors {
            let metadata = external(started, false, || fs::symlink_metadata(ancestor)).unwrap();
            ordinary(&metadata);
            assert!(metadata.is_dir(), "existing ordinary directory required");
        }
        let parent = external(started, false, || fs::canonicalize(&raw)).unwrap();
        let (receipt, _) = read_bounded(
            &parent.join("caller-receipt.json"),
            CALLER.len() + 1,
            started,
            false,
        );
        assert_eq!(receipt.bytes, CALLER);
        let nonce = external(started, false, nf_identity::keys::random_id)
            .expect("OS random nonce required");
        let nonce = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let pid = external(started, false, std::process::id);
        let root = parent.join(format!("actual-producer-{pid}-{sequence}-{nonce}"));
        clock(started, false);
        let created = fs::create_dir(&root);
        if created.is_ok() {
            *owned = Some(Self {
                root,
                started,
                entries: 0,
                bytes: 0,
                cleanup: false,
                stage: Stage::OwnedSetup,
                start_utc_millis,
                cutoff_utc_millis,
            });
        }
        // Persist ownership before this postcheck so caught expiry can retain its stage.
        clock(started, false);
        created.expect("fresh exclusive case directory required");
        owned.as_mut().unwrap().write("case.receipt.json", b"{\"version\":1,\"publicSyntheticOnly\":true,\"noSpawnByCase\":true,\"descendantZeroQualified\":false,\"checkedFileClose\":false,\"directorySyncQualified\":false,\"fullACLQualified\":false,\"atimeObserved\":false,\"metadataScope\":\"Unix_dev_ino_mode_nlink_uid_gid_readonly_len_mtime_ctime_or_Windows_attributes_creation_write_size_readonly\"}\n");
    }

    pub fn enter(&mut self, stage: Stage) {
        self.stage = stage;
        clock(self.started, false);
    }

    pub fn run<T>(&mut self, stage: Stage, operation: impl FnOnce() -> T) -> T {
        self.enter(stage);
        let result = operation();
        clock(self.started, false);
        result
    }

    pub fn write(&mut self, name: &str, bytes: &[u8]) {
        clock(self.started, self.cleanup);
        assert!(!name.is_empty() && name.len() <= 80);
        assert!(
            name.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
        );
        assert!(name != "." && name != "..");
        assert!(bytes.len() <= CAPTURE_BYTES, "capture byte limit");
        assert!(
            self.entries < CASE_ENTRIES && self.bytes + bytes.len() <= CASE_BYTES,
            "case retention limit"
        );
        let mut file = external(self.started, self.cleanup, || {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.root.join(name))
        })
        .unwrap();
        self.entries += 1;
        self.bytes += bytes.len();
        let mut remaining = bytes;
        while !remaining.is_empty() {
            let width = remaining.len().min(32768);
            let count = external(self.started, self.cleanup, || {
                file.write(&remaining[..width])
            })
            .unwrap();
            assert!(count > 0, "positive write required");
            remaining = &remaining[count..];
        }
        external(self.started, self.cleanup, || file.sync_all()).unwrap();
        external(self.started, self.cleanup, || drop(file));
        clock(self.started, self.cleanup);
    }

    pub fn snapshot(&self) -> BTreeMap<String, Entry> {
        let mut paths = Vec::new();
        let mut directory =
            external(self.started, self.cleanup, || fs::read_dir(&self.root)).unwrap();
        loop {
            let next = external(self.started, self.cleanup, || directory.next());
            let Some(entry) = next else {
                break;
            };
            assert!(paths.len() < CASE_ENTRIES, "case entry limit");
            let entry = entry.unwrap();
            let name = entry
                .file_name()
                .into_string()
                .expect("ASCII case leaf required");
            assert!(
                name.bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
            );
            paths.push((name, entry.path()));
        }
        external(self.started, self.cleanup, || drop(directory));
        clock(self.started, self.cleanup);
        let mut result = BTreeMap::new();
        let mut charged = 0;
        for (name, path) in paths {
            let allowance = CASE_BYTES - charged;
            let (entry, consumed) = read_bounded(&path, allowance, self.started, self.cleanup);
            charged += consumed;
            assert!(charged <= CASE_BYTES, "case read allowance");
            assert!(result.insert(name, entry).is_none());
        }
        clock(self.started, self.cleanup);
        result
    }

    pub fn finish(&mut self, passed: bool) {
        self.cleanup = true;
        clock(self.started, true);
        let outcome = if passed {
            "assertions_returned"
        } else {
            "assertion_unwind"
        };
        let stage = self.stage.label();
        // Retain closed last-attempted stage before inventory work can fail.
        self.write("outcome.receipt.json", format!("{{\"outcome\":\"{outcome}\",\"stage\":\"{stage}\",\"finalInventory\":\"pending\",\"descendantZeroQualified\":false,\"checkedFileClose\":false,\"directorySyncQualified\":false}}\n").as_bytes());
        let observed = external(self.started, true, || SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let observed = u64::try_from(observed).unwrap();
        assert!(observed >= self.start_utc_millis && observed <= self.cutoff_utc_millis);
        self.write("clock.receipt.json", format!("{{\"startUtcMillis\":{},\"cutoffUtcMillis\":{},\"observedUtcMillis\":{},\"caseLimitMillis\":20000}}\n", self.start_utc_millis, self.cutoff_utc_millis, observed).as_bytes());
        let before_inventory = self.snapshot();
        let mut inventory = String::from("PUBLIC-SYNTHETIC-CASE-INVENTORY-1\n");
        for (name, entry) in before_inventory {
            inventory.push_str(&format!(
                "{name} {} {}\n",
                entry.bytes.len(),
                entry.metadata
            ));
        }
        self.write("retained-originals.metadata.txt", inventory.as_bytes());
        clock(self.started, true);
    }
}
// No Drop cleanup: every created original or partial file remains caller-owned on failure.
