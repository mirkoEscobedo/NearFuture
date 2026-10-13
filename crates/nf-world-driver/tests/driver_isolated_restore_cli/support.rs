//! Retained synthetic public data only. RAM signers are never archived.
use super::miniature_support::{Fixture as MemoryFixture, claim};
use nf_identity::model::MembershipRepository;
use sha2::{Digest, Sha256};
use nf_store::miniature::{
    BootstrapSigner, ChallengeRequest, MiniatureKnownFrontiers, MiniatureReader,
    MiniatureSnapshot, MiniatureStore, ProofAttempt,
};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, time::Instant};
#[path = "capture.rs"]
mod capture;
#[path = "process.rs"]
mod process;
pub use capture::{binary_digest, receipt};
pub use process::run;

pub struct Fixture {
    pub data: PathBuf,
    pub control: PathBuf,
    pub source: PathBuf,
    pub backup: PathBuf,
    pub known: MiniatureKnownFrontiers,
    pub newer: MiniatureKnownFrontiers,
    pub snapshot: MiniatureSnapshot,
    pub state: Vec<u8>,
    membership: Vec<u8>,
    newest: MiniatureSnapshot,
    newest_state: Vec<u8>,
}

pub fn check(deadline: Instant) {
    assert!(Instant::now() < deadline, "single CLI case clock exhausted");
}

pub fn plain(path: &Path) -> fs::Metadata {
    let metadata = fs::symlink_metadata(path).unwrap();
    assert!(!metadata.file_type().is_symlink());
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(metadata.file_attributes() & 0x400, 0, "reparse point refused");
    }
    metadata
}

impl Fixture {
    pub fn new(deadline: Instant) -> Self {
        check(deadline);
        let parent = PathBuf::from(std::env::var_os("NF_ISOLATED_RESTORE_CLI_EVIDENCE_PARENT").expect("caller-owned CLI evidence parent required"));
        assert!(parent.is_absolute());
        for ancestor in parent.ancestors() {
            assert!(plain(ancestor).is_dir());
        }
        let parent = parent.canonicalize().unwrap();
        let nonce = capture::hex(&nf_identity::keys::random_id().unwrap());
        let root = parent.join(format!("isolated-cli-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).expect("exclusive fresh evidence directory; never reuse old artifacts");
        let data = root.join("data");
        let control = root.join("control");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&control).unwrap();
        capture::receipt(&control.join("custody.public"), "fresh caller-owned synthetic public database/capture tree; no keys, private vault, save, or deleting Drop");
        let source = data.join("original.sqlite");
        let backup = data.join("backup.sqlite");
        let mut f = MemoryFixture::new();
        let expected_membership = f.bytes();
        check(deadline);
        let mut writer = MiniatureStore::create(&source, f.spec, &f.bytes(), f.policy.clone(), &mut f.signer).unwrap();
        claim(&mut writer, &mut f);
        let known = writer.known_frontiers().unwrap();
        assert_eq!(known.storage.event_sequence.0, 0);
        assert_eq!(known.storage.store_revision, 1);
        assert_eq!(known.storage.membership_revision, Some(2));
        assert_eq!(known.minimum_authority_term.0, 1);
        let snapshot = writer.snapshot().unwrap();
        let state = writer.snapshot_bytes().unwrap();
        assert_eq!(writer.backup_to(&backup).unwrap(), known);
        // Advance ONLY this explicit RAM-owner fixture's original after taking
        // the backup. Independently retained newer minima never come from CLI
        // output or the archive; each must independently reject that archive.
        claim(&mut writer, &mut f);
        writer.prepare(vec![], vec![]).unwrap();
        let issued = writer.issue_challenge(ChallengeRequest::Activity {
            actor: f.policy.owner.account, device: f.policy.owner.device,
        }).unwrap();
        let proof = f.signer.sign(&issued.template).unwrap();
        writer.accept_activity(ProofAttempt { ticket: issued.ticket, proof }).unwrap();
        let batch = nf_kernel::miniature::settle_miniature(
            writer.world(), writer.pending().unwrap(), writer.authority().unwrap().context(),
        ).unwrap();
        let issued = writer.issue_challenge(ChallengeRequest::Commit).unwrap();
        let proof = f.signer.sign(&issued.template).unwrap();
        writer.commit(&batch, ProofAttempt { ticket: issued.ticket, proof }).unwrap();
        let mut membership = f.membership.clone();
        membership.revision += 1;
        membership.consumed.insert([99; 16]);
        writer.commit_membership(Some(f.membership.revision), &membership).unwrap();
        claim(&mut writer, &mut f);
        let newer = writer.known_frontiers().unwrap();
        assert!(newer.storage.event_sequence > known.storage.event_sequence);
        assert!(newer.storage.store_revision > known.storage.store_revision);
        assert!(newer.storage.membership_revision > known.storage.membership_revision);
        assert!(newer.minimum_authority_term > known.minimum_authority_term);
        let newest = writer.snapshot().unwrap();
        let newest_state = writer.snapshot_bytes().unwrap();
        drop(writer);
        drop(f); // Private RAM seeds never enter retained files or the command.
        check(deadline);
        capture::retained_bytes(&control.join("baseline-original.sqlite"), &capture::bounded(&source, 1_048_576));
        capture::retained_bytes(&control.join("baseline-backup.sqlite"), &capture::bounded(&backup, 1_048_576));
        capture::retained_bytes(&control.join("expected-backup-state.public"), &state);
        capture::retained_bytes(&control.join("expected-original-state.public"), &newest_state);
        capture::retained_bytes(&control.join("expected-backup-membership.public"), &expected_membership);
        capture::receipt(&control.join("expected-backup-membership.public.sha256"), &capture::hex(&Sha256::digest(&expected_membership)));
        capture::receipt(&control.join("independent-minima.public"), &format!("writer-held-before-archive={known:?};independently-advanced-original={newer:?};neither-derived-from-CLI-output"));
        Self { data, control, source, backup, known, newer, snapshot, state, membership: expected_membership, newest, newest_state }
    }

    pub fn assert_original(&self, deadline: Instant) {
        check(deadline);
        let current = MiniatureReader::open_existing(&self.source, self.newer).unwrap();
        assert!(current.is_read_only().unwrap());
        assert_eq!(current.snapshot(), &self.newest);
        assert_eq!(current.snapshot_bytes().unwrap(), self.newest_state);
        drop(current);
        let archive = MiniatureReader::open_existing(&self.backup, self.known).unwrap();
        assert!(archive.is_read_only().unwrap());
        assert_eq!(archive.snapshot(), &self.snapshot);
        assert_eq!(archive.snapshot_bytes().unwrap(), self.state);
        drop(archive);
        check(deadline);
    }

    pub fn assert_membership(&self, path: &Path, deadline: Instant) {
        check(deadline);
        let connection = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).unwrap();
        assert!(connection.is_readonly(rusqlite::DatabaseName::Main).unwrap());
        let count: i64 = connection.query_row("SELECT count(*) FROM membership", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
        let (body, digest): (Vec<u8>, Vec<u8>) = connection.query_row(
            "SELECT public_state,digest FROM membership", [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(body, self.membership);
        assert_eq!(digest, Sha256::digest(&self.membership).to_vec());
        connection.close().unwrap();
        check(deadline);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Entry { Directory, File(Vec<u8>) }

pub fn tree(root: &Path, deadline: Instant) -> BTreeMap<PathBuf, Entry> {
    let mut result = BTreeMap::new();
    let mut remaining = vec![root.to_path_buf()];
    let mut total = 0usize;
    while let Some(path) = remaining.pop() {
        check(deadline);
        let metadata = plain(&path);
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        if metadata.is_dir() {
            result.insert(relative, Entry::Directory);
            for entry in fs::read_dir(&path).unwrap() {
                remaining.push(entry.unwrap().path());
                assert!(remaining.len() <= 64);
            }
        } else {
            assert!(metadata.is_file() && metadata.len() <= 1_048_576);
            let bytes = capture::bounded(&path, 1_048_576);
            total = total.checked_add(bytes.len()).unwrap();
            assert!(total <= 16_777_216);
            result.insert(relative, Entry::File(bytes));
        }
        assert!(result.len() <= 64);
    }
    check(deadline);
    result
}
