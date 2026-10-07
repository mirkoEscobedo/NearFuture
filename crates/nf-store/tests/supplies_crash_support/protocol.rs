use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::{DeviceProof, Scope};
use nf_kernel::supplies::*;
use nf_store::supplies::KnownSuppliesFrontiers;
use std::{
    fs::{File, Metadata, OpenOptions},
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

pub const MANAGEMENT: Duration = Duration::from_secs(30);
pub const PREPARED: &[u8; 8] = b"PREPARED";
pub const BEFORE: &[u8; 8] = b"BEFORE01";
pub const GO: &[u8; 8] = b"GO000001";
const ISSUANCE_LEN: usize = 247;
const DESCRIPTOR_LEN: usize = 401;
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Cut {
    BeforeCall,
    AfterReturn,
}
pub struct Descriptor {
    pub cut: Cut,
    pub issue: Issuance,
    pub known: KnownSuppliesFrontiers,
    pub peer: Vec<u8>,
}
pub fn policy(issue: &Issuance) -> SuppliesPolicy {
    SuppliesPolicy {
        universe: issue.universe,
        history: issue.history,
        ruleset: [3; 32],
        issuers: vec![IssuerRule {
            issuer: issue.actor,
            content: SUPPLIES_CONTENT,
            origin: issue.origin,
            reason: IssuanceReason::AuthorityGrant,
            maximum: 1000,
        }],
        burners: vec![BurnRule {
            issuer: issue.actor,
            content: SUPPLIES_CONTENT,
            origin: issue.origin,
            reason: BurnReason::AuthorityDestruction,
            maximum: 1000,
        }],
    }
}
impl Descriptor {
    pub fn write(&self, root: &Path) {
        let body = issuance_bytes(&self.issue);
        assert_eq!(body.len(), ISSUANCE_LEN);
        assert!(!self.peer.is_empty() && self.peer.len() <= 128);
        let mut bytes = Vec::with_capacity(DESCRIPTOR_LEN);
        bytes.extend_from_slice(b"NFSCW001");
        bytes.push(match self.cut {
            Cut::BeforeCall => 1,
            Cut::AfterReturn => 2,
        });
        bytes.extend_from_slice(&self.known.revision.to_le_bytes());
        bytes.extend_from_slice(&self.known.membership_revision.to_le_bytes());
        bytes.extend_from_slice(&body);
        bytes.push(u8::try_from(self.peer.len()).unwrap());
        bytes.extend_from_slice(&self.peer);
        bytes.resize(DESCRIPTOR_LEN, 0);
        write_new(&root.join("descriptor"), &bytes);
    }
    pub fn read(root: &Path) -> Self {
        let bytes = read_frame::<DESCRIPTOR_LEN>(&root.join("descriptor"))
            .expect("worker descriptor must exist");
        assert_eq!(&bytes[..8], b"NFSCW001", "closed management version");
        let cut = match bytes[8] {
            1 => Cut::BeforeCall,
            2 => Cut::AfterReturn,
            _ => panic!("closed management cut"),
        };
        let issue = decode_issuance(&bytes[25..25 + ISSUANCE_LEN])
            .expect("canonical public issuance descriptor");
        let start = 25 + ISSUANCE_LEN;
        let length = usize::from(bytes[start]);
        assert!((1..=128).contains(&length), "bounded public peer");
        assert!(
            bytes[start + 1 + length..].iter().all(|b| *b == 0),
            "closed zero peer padding"
        );
        assert_eq!(
            issue.actor, issue.beneficiary,
            "closed own-stock worker profile"
        );
        assert_eq!(issue.content, SUPPLIES_CONTENT);
        assert_eq!(
            issue.origin,
            Origin {
                trust: TrustClass::Canonical,
                lineage: [4; 32]
            }
        );
        assert_eq!(issue.amount, 25);
        assert_eq!(issue.policy, policy_digest(&policy(&issue)).unwrap());
        let known = KnownSuppliesFrontiers {
            scope: Scope {
                universe: issue.universe,
                history: issue.history,
            },
            revision: u64::from_le_bytes(bytes[9..17].try_into().unwrap()),
            membership_revision: u64::from_le_bytes(bytes[17..25].try_into().unwrap()),
        };
        assert_eq!(
            (known.revision, known.membership_revision),
            (0, 0),
            "closed genesis worker floor"
        );
        Self {
            cut,
            issue,
            known,
            peer: bytes[start + 1..start + 1 + length].to_vec(),
        }
    }
}
fn no_links(metadata: &Metadata) {
    assert!(
        !metadata.file_type().is_symlink(),
        "management path must not be a symlink"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            metadata.file_attributes() & 0x400,
            0,
            "management path must not be a reparse point"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.is_file() {
            assert_eq!(metadata.nlink(), 1, "management file must have one link");
        }
    }
}
pub fn directory(path: &Path) {
    let metadata = std::fs::symlink_metadata(path)
        .unwrap_or_else(|_| panic!("owned management directory metadata"));
    no_links(&metadata);
    assert!(metadata.is_dir(), "owned management directory");
}
pub fn write_new(path: &Path, bytes: &[u8]) {
    assert!(
        !bytes.is_empty() && bytes.len() <= 512,
        "fixed management cap"
    );
    directory(path.parent().expect("owned frame parent"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|_| panic!("fresh management frame creation"));
    no_links(
        &file
            .metadata()
            .unwrap_or_else(|_| panic!("fresh frame metadata")),
    );
    file.write_all(bytes)
        .unwrap_or_else(|_| panic!("single management frame write"));
    file.sync_all()
        .unwrap_or_else(|_| panic!("single management frame sync"));
}
pub fn read_frame<const N: usize>(path: &Path) -> Option<[u8; N]> {
    assert!((1..=512).contains(&N), "fixed management frame bound");
    directory(path.parent().expect("owned frame parent"));
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => panic!("management frame metadata"),
    };
    no_links(&metadata);
    assert!(metadata.is_file(), "regular management frame");
    assert!(
        metadata.len() <= N as u64,
        "overlong management frame refused"
    );
    if metadata.len() < N as u64 {
        return None;
    } // Fresh writer has not yet published the whole fixed frame.
    let mut file = File::open(path).unwrap_or_else(|_| panic!("management frame open"));
    let opened = file
        .metadata()
        .unwrap_or_else(|_| panic!("opened management frame metadata"));
    no_links(&opened);
    assert!(
        opened.is_file() && opened.len() == N as u64,
        "exact opened management frame"
    );
    let mut bytes = [0; N];
    file.read_exact(&mut bytes)
        .unwrap_or_else(|_| panic!("complete management frame read"));
    let mut excess = [0; 1];
    assert_eq!(
        file.read(&mut excess)
            .unwrap_or_else(|_| panic!("management cap-plus-one read")),
        0
    );
    Some(bytes)
}
pub fn await_frame<const N: usize>(path: &Path, until: Instant) -> [u8; N] {
    loop {
        assert!(Instant::now() < until, "management prerequisite deadline");
        if let Some(bytes) = read_frame(path) {
            return bytes;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub fn encode_template(proof: &DeviceProof) -> [u8; 297] {
    assert!(!proof.peer.is_empty() && proof.peer.len() <= 128);
    assert_eq!(proof.signature, [0; 64]);
    let mut bytes = [0; 297];
    bytes[..16].copy_from_slice(proof.scope.universe.as_bytes());
    bytes[16..32].copy_from_slice(proof.scope.history.as_bytes());
    bytes[32..48].copy_from_slice(proof.account.as_bytes());
    bytes[48..64].copy_from_slice(proof.device.as_bytes());
    bytes[64..72].copy_from_slice(&proof.frontier.to_le_bytes());
    bytes[72] = u8::try_from(proof.peer.len()).unwrap();
    bytes[73..73 + proof.peer.len()].copy_from_slice(&proof.peer);
    bytes[201..233].copy_from_slice(&proof.challenge);
    bytes
}
pub fn decode_template(bytes: &[u8; 297]) -> DeviceProof {
    let length = usize::from(bytes[72]);
    assert!((1..=128).contains(&length));
    assert!(bytes[73 + length..201].iter().all(|b| *b == 0));
    assert!(
        bytes[233..].iter().all(|b| *b == 0),
        "unsigned actual template only"
    );
    DeviceProof {
        scope: Scope {
            universe: UniverseId::from_bytes(bytes[..16].try_into().unwrap()),
            history: HistoryId::from_bytes(bytes[16..32].try_into().unwrap()),
        },
        account: AccountId::from_bytes(bytes[32..48].try_into().unwrap()),
        device: DeviceId::from_bytes(bytes[48..64].try_into().unwrap()),
        frontier: u64::from_le_bytes(bytes[64..72].try_into().unwrap()),
        peer: bytes[73..73 + length].to_vec(),
        challenge: bytes[201..233].try_into().unwrap(),
        signature: [0; 64],
    }
}
pub fn returned(outcome: IssuanceOutcome) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[..8].copy_from_slice(b"RETURNED");
    bytes[8..24].copy_from_slice(outcome.issuance.as_bytes());
    bytes[24..].copy_from_slice(&outcome.revision.to_le_bytes());
    bytes
}

pub fn actual<T>(
    root: &Path,
    stage: u8,
    result: Result<T, nf_store::supplies::SuppliesStoreError>,
) -> T {
    use nf_store::supplies::SuppliesStoreError;
    match result {
        Ok(value) => value,
        Err(error) => {
            let (family, detail) = match error {
                SuppliesStoreError::Identity(e) => (1, e as u8),
                SuppliesStoreError::Storage(e) => (2, e as u8),
                SuppliesStoreError::Rejected(e) => (3, e as u8),
                SuppliesStoreError::Compact(_) => (4, 0),
                SuppliesStoreError::Corrupt => (5, 0),
                SuppliesStoreError::UnsupportedProfile => (6, 0),
                SuppliesStoreError::StaleBackup => (7, 0),
                SuppliesStoreError::Replay => (8, 0),
                SuppliesStoreError::Expired => (9, 0),
                SuppliesStoreError::Entropy => (10, 0),
            };
            let mut frame = [0; 16];
            frame[..8].copy_from_slice(b"NFSCFAIL");
            frame[8] = stage;
            frame[9] = family;
            frame[10] = detail;
            write_new(&root.join("failure"), &frame);
            panic!("actual worker public operation refused; closed failure frame preserved");
        }
    }
}
pub fn failure(root: &Path) -> Option<[u8; 3]> {
    read_frame::<16>(&root.join("failure")).map(|frame| {
        assert_eq!(&frame[..8], b"NFSCFAIL");
        assert!((1..=4).contains(&frame[8]));
        assert!((1..=10).contains(&frame[9]));
        assert!(frame[11..].iter().all(|b| *b == 0));
        match frame[9] {
            1 => assert!(frame[10] <= 16),
            2 => assert!(frame[10] <= 14),
            3 => assert!(frame[10] <= 6),
            _ => assert_eq!(frame[10], 0),
        }
        [frame[8], frame[9], frame[10]]
    })
}
