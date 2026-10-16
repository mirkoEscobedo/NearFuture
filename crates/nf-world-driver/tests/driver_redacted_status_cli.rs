use nf_contract::identity::*;
use nf_identity::{
    keys::{SecretSeed, generate_identity, random_id},
    model::{DeviceProof, Invitation, MembershipState, Roles, Scope},
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_store::miniature::{
    AuthConfig, BootstrapPolicy, BootstrapSigner, MiniatureGenesisSpec, MiniatureStore,
    MiniatureStoreError,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const CASE_LIMIT: Duration = Duration::from_secs(20);
const CHILD_CLEANUP_LIMIT: Duration = Duration::from_secs(2);
const STDOUT_LIMIT: u64 = 4096;
const STDERR_LIMIT: u64 = 1024;
const DATABASE_LIMIT: u64 = 1_048_576;
// Match the retained debug-symbol artifact cap; ELF embeds DWARF in the binary.
const BINARY_LIMIT: u64 = 268_435_456;
const BINARY_HASH_CHUNK: usize = 65_536;

struct MemorySigner {
    public: nf_identity::model::PublicIdentity,
    device: SecretSeed,
}
impl BootstrapSigner for MemorySigner {
    fn sign(&mut self, template: &DeviceProof) -> Result<DeviceProof, MiniatureStoreError> {
        if template.account != self.public.account
            || template.device != self.public.device
            || template.peer != self.public.peer
        {
            return Err(MiniatureStoreError::Unauthorized);
        }
        let mut proof = template.clone();
        proof.signature = self.device.sign(&device_digest(&proof)?);
        Ok(proof)
    }
}

// Retain every fresh fixture, including a passing one. No Drop deletes evidence.
struct Fixture {
    root: PathBuf,
    database: PathBuf,
    known: nf_store::miniature::MiniatureKnownFrontiers,
    snapshot: nf_store::miniature::MiniatureSnapshot,
}

fn plain(path: &Path) {
    let metadata = fs::symlink_metadata(path).expect("owned metadata");
    assert!(!metadata.file_type().is_symlink(), "symlink refused");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            metadata.file_attributes() & 0x400,
            0,
            "reparse point refused"
        );
    }
}

fn owned_root() -> PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace");
    for ancestor in workspace.ancestors() {
        plain(ancestor);
    }
    let parent = workspace.join(".tmp");
    match fs::create_dir(&parent) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => panic!("owned fixture parent: {error}"),
    }
    plain(&parent);
    assert!(parent.is_dir());
    let suffix = hex(&random_id().expect("fresh fixture identity"));
    let target = parent.join(format!("operator46-redacted-cli-{suffix}"));
    assert_eq!(target.parent(), Some(parent.as_path()));
    fs::create_dir(&target).expect("exclusive fresh fixture directory");
    plain(&target);
    assert_eq!(target.canonicalize().unwrap(), target);
    target
}

fn fixture(deadline: Instant) -> Fixture {
    let root = owned_root();
    let database = root.join("history.sqlite");
    let scope = Scope {
        universe: UniverseId::from_bytes([0x81; 16]),
        history: HistoryId::from_bytes([0x82; 16]),
    };
    let (owner, owner_account, owner_device) =
        generate_identity(b"operator46-owner".to_vec()).unwrap();
    let mut membership = MembershipState::bootstrap(scope, &owner).unwrap();
    let mut controllers = vec![owner.account];
    for ordinal in 1..=2u8 {
        let (public, account, device) = generate_identity(vec![ordinal]).unwrap();
        let invitation = Invitation {
            scope,
            id: [ordinal; 16],
            issuer: owner.account,
            recipient: public.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: membership.revision,
            reusable: false,
        };
        let proof = admission_proof(&invitation, &account, &device).unwrap();
        let signed = sign_invitation(invitation, &owner_account).unwrap();
        membership = membership.redeem(&signed, &proof, 1).unwrap();
        controllers.push(public.account);
    }
    let controllers: [AccountId; 3] = controllers.try_into().unwrap();
    let body = nf_identity::codec::encode_state(&membership).unwrap();
    let spec = MiniatureGenesisSpec {
        genesis: nf_world::Genesis {
            universe: scope.universe,
            history: scope.history,
            seed: [0x83; 32],
            accounts: controllers,
        },
        aggregate: AggregateId::from_bytes([0x84; 16]),
        provider: ProviderId::from_bytes([0x85; 16]),
        provider_aggregate: AggregateId::from_bytes([0x86; 16]),
    };
    let policy = BootstrapPolicy {
        scope,
        membership_digest: Sha256::digest(&body).into(),
        owner: owner.clone(),
        controllers,
        auth: AuthConfig::default(),
    };
    let mut signer = MemorySigner {
        public: owner,
        device: owner_device,
    };
    assert!(Instant::now() < deadline, "fixture stage deadline");
    let store = MiniatureStore::create(&database, spec, &body, policy, &mut signer).unwrap();
    let snapshot = store.snapshot().unwrap();
    assert!(snapshot.authority.is_none());
    assert!(snapshot.pending.is_none());
    assert_eq!(snapshot.known.storage.membership_revision, Some(2));
    let known = snapshot.known;
    drop(store); // Release the initial EXCLUSIVE owner before the CLI opens it.
    assert!(Instant::now() < deadline, "fixture close deadline");
    Fixture {
        root,
        database,
        known,
        snapshot,
    }
}

fn new_file(path: &Path) -> File {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap()
}

fn bounded(path: &Path, maximum: u64) -> Vec<u8> {
    plain(path);
    let file = File::open(path).unwrap();
    assert!(
        file.metadata().unwrap().len() <= maximum,
        "capture/file cap"
    );
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() as u64 <= maximum, "bounded read cap");
    bytes
}

fn same_binary_metadata(before: &Metadata, after: &Metadata) {
    assert!(
        before.is_file() && after.is_file(),
        "Cargo binary regular file"
    );
    assert_eq!(before.len(), after.len(), "Cargo binary size changed");
    assert_eq!(
        before.modified().unwrap(),
        after.modified().unwrap(),
        "Cargo binary modification changed"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            (
                before.dev(),
                before.ino(),
                before.ctime(),
                before.ctime_nsec()
            ),
            (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()),
            "Cargo binary identity changed"
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            (
                before.file_attributes(),
                before.creation_time(),
                before.last_write_time()
            ),
            (
                after.file_attributes(),
                after.creation_time(),
                after.last_write_time()
            ),
            "Cargo binary metadata changed"
        );
    }
}

fn binary_digest(path: &Path, deadline: Instant) -> String {
    assert!(Instant::now() < deadline, "before binary diagnostic");
    plain(path);
    let path_before = fs::metadata(path).expect("Cargo binary metadata");
    let mut file = File::open(path).expect("Cargo binary diagnostic");
    let before = file.metadata().expect("held Cargo binary metadata");
    same_binary_metadata(&path_before, &before);
    assert!(
        before.len() > 0 && before.len() <= BINARY_LIMIT,
        "portable Cargo debug binary cap"
    );
    let mut digest = Sha256::new();
    let mut buffer = [0u8; BINARY_HASH_CHUNK];
    let mut bytes_read = 0u64;
    while bytes_read < before.len() {
        assert!(Instant::now() < deadline, "binary diagnostic read deadline");
        let count = (before.len() - bytes_read).min(BINARY_HASH_CHUNK as u64) as usize;
        let read = file
            .read(&mut buffer[..count])
            .expect("bounded binary diagnostic read");
        assert!(read > 0, "Cargo binary shortened");
        digest.update(&buffer[..read]);
        bytes_read += read as u64;
        assert!(
            Instant::now() < deadline,
            "binary diagnostic chunk deadline"
        );
    }
    let mut extra = [0u8; 1];
    assert!(Instant::now() < deadline, "binary diagnostic EOF deadline");
    assert_eq!(file.read(&mut extra).unwrap(), 0, "Cargo binary grew");
    same_binary_metadata(&before, &file.metadata().unwrap());
    plain(path);
    same_binary_metadata(&before, &fs::metadata(path).unwrap());
    drop(file);
    let result = hex(&digest.finalize());
    assert!(
        Instant::now() < deadline,
        "complete binary diagnostic deadline"
    );
    result
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

struct OwnedChild {
    child: Child,
    reaped: bool,
    termination_requested: bool,
    cleanup_deadline: Option<Instant>,
    cleanup_receipt: PathBuf,
}
impl OwnedChild {
    // Own the direct Cargo-built foreground status process through its returned
    // process handle. This closed CLI branch launches no descendants.
    fn terminate_and_reap(&mut self) -> std::io::Result<ExitStatus> {
        let deadline = *self
            .cleanup_deadline
            .get_or_insert_with(|| Instant::now() + CHILD_CLEANUP_LIMIT);
        if !self.termination_requested {
            self.termination_requested = true;
            let _ = self.child.kill();
        }
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.reaped = true;
                return Ok(status);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "owned CLI kill/reap deadline",
                ));
            }
            std::thread::sleep(remaining.min(Duration::from_millis(10)));
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            let result = self.terminate_and_reap();
            if let Ok(mut receipt) = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.cleanup_receipt)
            {
                let _ = writeln!(
                    receipt,
                    "owned_handle_cleanup={:?};reaped={}",
                    result
                        .as_ref()
                        .map(ExitStatus::code)
                        .map_err(std::io::Error::kind),
                    self.reaped
                );
                let _ = receipt.sync_all();
            }
        }
    }
}

fn finish_child(
    owned: &mut OwnedChild,
    stdout: &Path,
    stderr: &Path,
    deadline: Instant,
) -> ExitStatus {
    loop {
        let over = fs::metadata(stdout).unwrap().len() > STDOUT_LIMIT
            || fs::metadata(stderr).unwrap().len() > STDERR_LIMIT;
        let expired = Instant::now() >= deadline;
        if over || expired {
            let termination = owned.terminate_and_reap();
            let mut receipt = new_file(&stdout.with_file_name("failure.public"));
            writeln!(
                receipt,
                "output_cap={over};deadline={expired};reaped={};exit={:?}",
                owned.reaped,
                termination
                    .as_ref()
                    .map(ExitStatus::code)
                    .map_err(std::io::Error::kind)
            )
            .unwrap();
            receipt.sync_all().unwrap();
            panic!("owned CLI refused: bounded captures and fixture retained");
        }
        match owned.child.try_wait() {
            Ok(Some(status)) => {
                owned.reaped = true;
                return status;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => panic!("owned CLI observation failed: captures retained"),
        }
    }
}

#[test]
fn actual_redacted_cli_routes_closed_json_and_preserves_fresh_profile2_without_vaults() {
    let deadline = Instant::now() + CASE_LIMIT;
    let executable = Path::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    // Cargo selects the current package binary. This bounded streaming diagnostic
    // is separate from the independent literal JSON behavior oracle.
    let expected_binary = binary_digest(executable, deadline);
    let fixture = fixture(deadline);
    let before_database = bounded(&fixture.database, DATABASE_LIMIT);
    let stdout = fixture.root.join("cli.stdout.raw");
    let stderr = fixture.root.join("cli.stderr.raw");
    let mut receipt = new_file(&fixture.root.join("binary.public.sha256"));
    writeln!(receipt, "{expected_binary}").unwrap();
    receipt.sync_all().unwrap();
    drop(receipt);
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(new_file(&stdout)))
        .stderr(Stdio::from(new_file(&stderr)))
        .arg("status-redacted")
        .arg("--database")
        .arg(&fixture.database)
        .args([
            "--universe",
            &hex(fixture.known.storage.scope.universe.as_bytes()),
            "--history",
            &hex(fixture.known.storage.scope.history.as_bytes()),
            "--min-event",
            "0",
            "--min-store",
            "0",
            "--min-membership",
            "2",
            "--min-term",
            "0",
        ]);
    assert!(Instant::now() < deadline, "before child deadline");
    let mut owned = OwnedChild {
        child: command.spawn().expect("one owned CLI child"),
        reaped: false,
        termination_requested: false,
        cleanup_deadline: None,
        cleanup_receipt: fixture.root.join("cleanup.public"),
    };
    drop(command); // Close parent copies of the redirected child handles.
    let status = finish_child(&mut owned, &stdout, &stderr, deadline);
    drop(owned);
    assert_eq!(
        binary_digest(executable, deadline),
        expected_binary,
        "Cargo-produced CLI bytes changed during the case"
    );
    let output = bounded(&stdout, STDOUT_LIMIT);
    let errors = bounded(&stderr, STDERR_LIMIT);
    assert!(status.success(), "CLI failed; original captures retained");
    assert!(errors.is_empty(), "unexpected stderr retained");
    // Independently stated complete schema/values, never generated by the
    // projection implementation. It also excludes scope/IDs/hash/keys/paths.
    assert_eq!(
        output,
        concat!(
            "{\"format\":\"NF-OPERATOR-MINIATURE-STATUS-1\",\"store_profile\":\"profile2\",",
            "\"world_tick\":\"0\",\"event_sequence\":\"0\",\"store_revision\":\"0\",",
            "\"membership_revision\":\"2\",\"persisted_authority_term\":null,",
            "\"minimum_known_authority_term\":\"0\",\"provider_revision\":\"0\",",
            "\"pending_frontier\":false,\"pending_intents_count\":\"0\",",
            "\"provider_ownership\":\"unavailable\",\"lag\":\"unavailable\",",
            "\"taint\":\"unavailable\",\"pending_native_bridge\":\"unavailable\",",
            "\"transfer_receipts\":\"unavailable_profile2\",\"backup_recovery\":\"unavailable\"}"
        )
        .as_bytes()
    );
    assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);
    assert!(
        Instant::now() < deadline,
        "before verification reopen deadline"
    );
    let reopened =
        MiniatureStore::open_existing(&fixture.database, fixture.known, AuthConfig::default())
            .unwrap();
    assert_eq!(reopened.snapshot().unwrap(), fixture.snapshot);
    drop(reopened);
    assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);
    assert!(Instant::now() < deadline, "verification reopen deadline");
    let mut success = new_file(&fixture.root.join("success.public"));
    writeln!(success, "cli_reaped=true;stdout_bytes={};stderr_bytes=0;database_bytes_held=true;fresh_fixture_retained=true",
        output.len()).unwrap();
    success.sync_all().unwrap();
    drop(success);
    assert!(Instant::now() < deadline, "complete CLI receipt deadline");
}

struct RestoreDatabasePermissions(PathBuf, fs::Permissions);
impl Drop for RestoreDatabasePermissions {
    fn drop(&mut self) {
        fs::set_permissions(&self.0, self.1.clone()).expect("restore owned database permissions");
    }
}

fn directory_names(path: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    names
}

fn status_options(
    fixture: &Fixture,
    known: nf_store::miniature::MiniatureKnownFrontiers,
) -> nf_world_driver::OpenOptions {
    nf_world_driver::OpenOptions {
        database: fixture.database.clone(),
        scope: known.storage.scope,
        minimum_event: known.storage.event_sequence,
        minimum_store: known.storage.store_revision,
        minimum_membership: known.storage.membership_revision.unwrap(),
        minimum_term: known.minimum_authority_term,
    }
}

#[test]
fn redacted_status_reads_read_only_media_and_preserves_all_protected_minima() {
    let deadline = Instant::now() + CASE_LIMIT;
    let executable = Path::new(env!("CARGO_BIN_EXE_nf-world-driver"));
    let expected_binary = binary_digest(executable, deadline);
    let fixture = fixture(deadline);
    let before_database = bounded(&fixture.database, DATABASE_LIMIT);
    let capture_root = fixture.root.join("read-only-captures");
    fs::create_dir(&capture_root).unwrap();
    let mut captures = Vec::new();
    for index in 0..5 {
        let stdout = capture_root.join(format!("{index}.stdout.raw"));
        let stderr = capture_root.join(format!("{index}.stderr.raw"));
        // Reserve all output names before observing the database directory.
        let out = new_file(&stdout);
        let err = new_file(&stderr);
        out.sync_all().unwrap();
        err.sync_all().unwrap();
        captures.push((stdout, stderr, out, err));
    }
    let before_directory = directory_names(&fixture.root);
    let permissions = fs::metadata(&fixture.database).unwrap().permissions();
    let restore = RestoreDatabasePermissions(fixture.database.clone(), permissions.clone());
    let mut read_only = permissions;
    read_only.set_readonly(true);
    fs::set_permissions(&fixture.database, read_only).unwrap();

    let reader =
        nf_world_driver::Driver::read_only_status(status_options(&fixture, fixture.known)).unwrap();
    assert!(
        reader.is_read_only().unwrap(),
        "actual SQLite READ_ONLY mode"
    );
    assert_eq!(reader.snapshot(), &fixture.snapshot);
    let expected_state = reader.snapshot_bytes().unwrap();
    drop(reader);
    assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);

    for (index, (stdout, stderr, out, err)) in captures.into_iter().enumerate() {
        let mut known = fixture.known;
        if index != 0 {
            match index {
                1 => known.storage.event_sequence = EventSeq(1),
                2 => known.storage.store_revision = 1,
                3 => known.storage.membership_revision = Some(3),
                _ => known.minimum_authority_term = AuthorityTerm(1),
            }
            assert_eq!(
                nf_world_driver::Driver::read_only_status(status_options(&fixture, known)).err(),
                Some(nf_world_driver::DriverError::Store(
                    MiniatureStoreError::Storage(nf_store::StoreError::StaleBackup)
                ))
            );
        }
        let mut command = Command::new(executable);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err))
            .arg("status-redacted")
            .arg("--database")
            .arg(&fixture.database)
            .args([
                "--universe",
                &hex(known.storage.scope.universe.as_bytes()),
                "--history",
                &hex(known.storage.scope.history.as_bytes()),
                "--min-event",
                &known.storage.event_sequence.0.to_string(),
                "--min-store",
                &known.storage.store_revision.to_string(),
                "--min-membership",
                &known.storage.membership_revision.unwrap().to_string(),
                "--min-term",
                &known.minimum_authority_term.0.to_string(),
            ]);
        assert!(Instant::now() < deadline, "before read-only CLI deadline");
        let mut owned = OwnedChild {
            child: command.spawn().expect("owned read-only status child"),
            reaped: false,
            termination_requested: false,
            cleanup_deadline: None,
            cleanup_receipt: capture_root.join(format!("{index}.cleanup.public")),
        };
        drop(command);
        let status = finish_child(&mut owned, &stdout, &stderr, deadline);
        drop(owned);
        let output = bounded(&stdout, STDOUT_LIMIT);
        let errors = bounded(&stderr, STDERR_LIMIT);
        if index == 0 {
            assert_eq!(status.code(), Some(0), "read-only CLI status failed");
            assert!(errors.is_empty(), "read-only CLI stderr retained");
            assert_eq!(
                output,
                concat!(
                    "{\"format\":\"NF-OPERATOR-MINIATURE-STATUS-1\",\"store_profile\":\"profile2\",",
                    "\"world_tick\":\"0\",\"event_sequence\":\"0\",\"store_revision\":\"0\",",
                    "\"membership_revision\":\"2\",\"persisted_authority_term\":null,",
                    "\"minimum_known_authority_term\":\"0\",\"provider_revision\":\"0\",",
                    "\"pending_frontier\":false,\"pending_intents_count\":\"0\",",
                    "\"provider_ownership\":\"unavailable\",\"lag\":\"unavailable\",",
                    "\"taint\":\"unavailable\",\"pending_native_bridge\":\"unavailable\",",
                    "\"transfer_receipts\":\"unavailable_profile2\",\"backup_recovery\":\"unavailable\"}"
                )
                .as_bytes()
            );
        } else {
            assert_eq!(status.code(), Some(1), "stale minimum must fail");
            assert!(output.is_empty(), "stale minimum must publish no JSON");
            assert_eq!(errors, b"driver: Store(Storage(StaleBackup))\n");
        }
        assert!(
            fs::metadata(&fixture.database)
                .unwrap()
                .permissions()
                .readonly()
        );
        assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);
        assert_eq!(directory_names(&fixture.root), before_directory);
        assert!(
            Instant::now() < deadline,
            "complete read-only invocation deadline"
        );
    }
    let reader =
        nf_world_driver::Driver::read_only_status(status_options(&fixture, fixture.known)).unwrap();
    assert!(reader.is_read_only().unwrap());
    assert_eq!(reader.snapshot(), &fixture.snapshot);
    assert_eq!(reader.snapshot_bytes().unwrap(), expected_state);
    drop(reader);
    assert_eq!(bounded(&fixture.database, DATABASE_LIMIT), before_database);
    assert_eq!(directory_names(&fixture.root), before_directory);
    assert_eq!(binary_digest(executable, deadline), expected_binary);
    drop(restore);
    assert!(
        Instant::now() < deadline,
        "complete read-only status case deadline"
    );
}

#[path = "driver_redacted_status_cli/support_preview.rs"]
mod support_preview;
