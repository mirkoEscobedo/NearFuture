use nf_contract::identity::{AccountId, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_store::chat::{
    ChatStoreError,
    local_preferences::{load_local_mutes, save_new_local_mutes},
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

// These fixed metadata values confer no membership or signing authority. The unchanged
// physical FIRST separately establishes genuine authenticated SQL/history preservation.
struct OwnedMetadataFiles {
    root: PathBuf,
    scope: Scope,
    viewer: AccountId,
    muted: AccountId,
}

impl OwnedMetadataFiles {
    fn new(case: &str) -> Self {
        let evidence = PathBuf::from(
            std::env::var_os("NF_CHAT_LOCAL_MUTE_PREFERENCES_EVIDENCE_DIR")
                .expect("Root must supply one fresh owned external evidence directory"),
        );
        assert!(evidence.is_absolute());
        let evidence = evidence.canonicalize().unwrap();
        assert!(evidence.is_dir());
        let root = evidence.join(case);
        assert!(!root.exists());
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        assert_eq!(root.parent(), Some(evidence.as_path()));
        Self {
            root,
            scope: Scope {
                universe: UniverseId::from_bytes([1; 16]),
                history: HistoryId::from_bytes([2; 16]),
            },
            viewer: AccountId::from_bytes([4; 16]),
            muted: AccountId::from_bytes([3; 16]),
        }
    }
}

// Caller-owned metadata originals remain on success and panic/unwind; no Drop removes them.
fn owned_path(fixture: &OwnedMetadataFiles, name: &str) -> PathBuf {
    fixture.root.join(name)
}

fn literal(scope: Scope, viewer: AccountId, accounts: &[AccountId]) -> Vec<u8> {
    let mut bytes = b"NF-MUTE1".to_vec();
    bytes.extend_from_slice(scope.universe.as_bytes());
    bytes.extend_from_slice(scope.history.as_bytes());
    bytes.extend_from_slice(viewer.as_bytes());
    bytes.push(accounts.len() as u8);
    for account in accounts {
        bytes.extend_from_slice(account.as_bytes());
    }
    bytes
}

fn create_original(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn refused_unchanged(path: &Path, scope: Scope, viewer: AccountId, error: ChatStoreError) {
    let before = fs::read(path).unwrap();
    assert_eq!(load_local_mutes(path, scope, viewer), Err(error));
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn expected_universe_history_and_viewer_are_not_adopted_from_preferences() {
    let fixture = OwnedMetadataFiles::new("controls-scope");
    let scope = fixture.scope;
    let viewer = fixture.viewer;
    let path = owned_path(&fixture, "scope.original.bin");
    let accounts = [fixture.muted];
    save_new_local_mutes(&path, scope, viewer, &accounts).unwrap();
    let expected = literal(scope, viewer, &accounts);
    assert_eq!(fs::read(&path).unwrap(), expected);
    refused_unchanged(
        &path,
        Scope {
            universe: UniverseId::from_bytes([9; 16]),
            history: scope.history,
        },
        viewer,
        ChatStoreError::Scope,
    );
    refused_unchanged(
        &path,
        Scope {
            universe: scope.universe,
            history: HistoryId::from_bytes([9; 16]),
        },
        viewer,
        ChatStoreError::Scope,
    );
    refused_unchanged(&path, scope, fixture.muted, ChatStoreError::Scope);
    assert_eq!(
        load_local_mutes(&path, scope, viewer),
        Ok(accounts.to_vec())
    );
}

#[test]
fn empty_and_raw64_preserve_duplicate_order_while65_does_not_create() {
    let fixture = OwnedMetadataFiles::new("controls-raw-cap");
    let scope = fixture.scope;
    let viewer = fixture.viewer;
    let empty = owned_path(&fixture, "empty.original.bin");
    save_new_local_mutes(&empty, scope, viewer, &[]).unwrap();
    assert_eq!(fs::read(&empty).unwrap(), literal(scope, viewer, &[]));
    assert_eq!(load_local_mutes(&empty, scope, viewer), Ok(Vec::new()));
    let raw: Vec<_> = (0..64)
        .map(|index| {
            if index % 3 == 0 {
                viewer
            } else {
                fixture.muted
            }
        })
        .collect();
    let maximum = owned_path(&fixture, "raw64.original.bin");
    save_new_local_mutes(&maximum, scope, viewer, &raw).unwrap();
    let expected = literal(scope, viewer, &raw);
    assert_eq!(expected.len(), 1081);
    assert_eq!(fs::read(&maximum).unwrap(), expected);
    assert_eq!(load_local_mutes(&maximum, scope, viewer), Ok(raw.clone()));
    let excessive = owned_path(&fixture, "raw65.absent.bin");
    let mut raw65 = raw;
    raw65.push(fixture.muted);
    assert_eq!(
        save_new_local_mutes(&excessive, scope, viewer, &raw65),
        Err(ChatStoreError::Limit)
    );
    assert!(!excessive.exists());
}

#[test]
fn zero_input_ids_refuse_before_create_and_stored_zero_account_is_corrupt() {
    let fixture = OwnedMetadataFiles::new("controls-zero");
    let scope = fixture.scope;
    let viewer = fixture.viewer;
    let zero = AccountId::from_bytes([0; 16]);
    let zero_viewer = owned_path(&fixture, "zero-viewer.absent.bin");
    assert_eq!(
        save_new_local_mutes(&zero_viewer, scope, zero, &[]),
        Err(ChatStoreError::Malformed)
    );
    assert!(!zero_viewer.exists());
    let zero_account = owned_path(&fixture, "zero-account.absent.bin");
    assert_eq!(
        save_new_local_mutes(&zero_account, scope, viewer, &[zero]),
        Err(ChatStoreError::Malformed)
    );
    assert!(!zero_account.exists());
    let stored_zero = owned_path(&fixture, "stored-zero.original.bin");
    create_original(&stored_zero, &literal(scope, viewer, &[zero]));
    refused_unchanged(&stored_zero, scope, viewer, ChatStoreError::Corrupt);
    let valid = owned_path(&fixture, "valid.original.bin");
    create_original(&valid, &literal(scope, viewer, &[fixture.muted]));
    refused_unchanged(&valid, scope, zero, ChatStoreError::Malformed);
}

#[test]
fn malformed_magic_version_truncation_count_and_trailing_bytes_do_not_reset() {
    let fixture = OwnedMetadataFiles::new("controls-corrupt");
    let scope = fixture.scope;
    let viewer = fixture.viewer;
    let good = literal(scope, viewer, &[fixture.muted]);
    let mut magic = good.clone();
    magic[0] = b'X';
    let mut version = good.clone();
    version[7] = b'2';
    let mut count_zero = good.clone();
    count_zero[56] = 0;
    let mut count_two = good.clone();
    count_two[56] = 2;
    let mut trailing = good.clone();
    trailing.push(0);
    let cases = [
        magic,
        version,
        Vec::new(),
        good[..56].to_vec(),
        good[..72].to_vec(),
        count_zero,
        count_two,
        trailing,
    ];
    for (index, bytes) in cases.into_iter().enumerate() {
        let path = owned_path(&fixture, &format!("corrupt-{index}.original.bin"));
        create_original(&path, &bytes);
        refused_unchanged(&path, scope, viewer, ChatStoreError::Corrupt);
    }
}

#[test]
fn oversized_file_and_count_refuse_without_modifying_the_original() {
    let fixture = OwnedMetadataFiles::new("controls-bounded");
    let scope = fixture.scope;
    let viewer = fixture.viewer;
    let raw64 = vec![fixture.muted; 64];
    let mut oversized = literal(scope, viewer, &raw64);
    assert_eq!(oversized.len(), 1081);
    oversized.push(0);
    assert_eq!(oversized.len(), 1082);
    let path = owned_path(&fixture, "1082.original.bin");
    create_original(&path, &oversized);
    refused_unchanged(&path, scope, viewer, ChatStoreError::Limit);
    let mut large = oversized;
    large.resize(4096, 0);
    let large_path = owned_path(&fixture, "4096.original.bin");
    create_original(&large_path, &large);
    refused_unchanged(&large_path, scope, viewer, ChatStoreError::Limit);
    let mut count65 = literal(scope, viewer, &[]);
    count65[56] = 65;
    let count_path = owned_path(&fixture, "count65.original.bin");
    create_original(&count_path, &count65);
    refused_unchanged(&count_path, scope, viewer, ChatStoreError::Limit);
}

#[test]
fn missing_paths_and_owned_directory_io_fail_explicitly_without_fallback() {
    let fixture = OwnedMetadataFiles::new("controls-storage");
    let scope = fixture.scope;
    let viewer = fixture.viewer;
    let missing = owned_path(&fixture, "missing.absent.bin");
    assert_eq!(
        load_local_mutes(&missing, scope, viewer),
        Err(ChatStoreError::Storage)
    );
    assert!(!missing.exists());
    let directory = owned_path(&fixture, "owned-directory");
    fs::create_dir(&directory).unwrap();
    assert_eq!(
        load_local_mutes(&directory, scope, viewer),
        Err(ChatStoreError::Storage)
    );
    assert!(directory.is_dir());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
    let absent_parent = owned_path(&fixture, "absent-parent").join("preferences.bin");
    assert_eq!(
        save_new_local_mutes(&absent_parent, scope, viewer, &[]),
        Err(ChatStoreError::Storage)
    );
    assert!(!absent_parent.exists());
    assert!(!absent_parent.parent().unwrap().exists());
}
