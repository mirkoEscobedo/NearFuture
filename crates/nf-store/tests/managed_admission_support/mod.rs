#![allow(dead_code)] // Shared by separate public admission and provider-bound targets.
use super::support;
use nf_contract::identity::{
    AccountId, AggregateId, AggregateRevision, DeviceId, EventSeq, ProviderId, WorldTick,
};
use nf_identity::{
    keys::SecretSeed,
    model::{DeviceProof, MembershipRepository, MembershipState, PublicIdentity},
    signing::device_digest,
};
use nf_kernel::{MissingPolicy, OwnershipRecord, ProviderState, Settlement, World};
use nf_store::{
    KnownFrontiers, Store,
    managed::{
        AdoptionDraft, AdoptionFailure, AdoptionInput, BinaryEvidence, FilePin, LocalCoordinator,
        ManagedDrainedSource, QuiescePolicy, SourceOwner,
    },
    migration::{Composition, ProviderVersion},
};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const SOURCE_SCHEMA: [u8; 32] = [
    0x7e, 0x06, 0x49, 0x12, 0x04, 0xd7, 0xaf, 0xa6, 0x93, 0x5f, 0x76, 0xf7, 0x69, 0xcb, 0x3f, 0xbf,
    0x98, 0x1c, 0x8b, 0x7c, 0x9c, 0x65, 0x10, 0x06, 0x73, 0xce, 0x3e, 0x32, 0x01, 0x23, 0x6e, 0xbb,
];
const TARGET_SCHEMA: [u8; 32] = [
    0x4a, 0x2c, 0xb2, 0x06, 0x36, 0x9d, 0x17, 0xa3, 0xf4, 0xc1, 0x34, 0x60, 0x4e, 0x01, 0xda, 0x01,
    0x56, 0x65, 0x73, 0x1b, 0xaa, 0x4d, 0xcb, 0x29, 0x92, 0xab, 0x8e, 0xc9, 0x91, 0x41, 0x93, 0xd3,
];
const PROVENANCE: &str = "TEST_EXECUTABLE_BYTES; MANAGED_RUNTIME_NOT_QUALIFIED";

pub struct Fixture {
    pub scratch: support::Scratch,
    pub known: KnownFrontiers,
    pub public: PublicIdentity,
    pub account_key: SecretSeed,
    pub device_key: SecretSeed,
    pub from: Composition,
    pub to: Composition,
    pub supported: Vec<ProviderVersion>,
    pub old: FilePin,
    pub new: FilePin,
    pub cutoff: u64,
}
impl Fixture {
    pub fn new(provider_count: usize) -> Self {
        let cutoff = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
        .checked_add(15_000)
        .unwrap();
        assert!((1..=32).contains(&provider_count));
        let scratch = support::Scratch::new();
        let account_key = SecretSeed::generate().unwrap();
        let device_key = SecretSeed::generate().unwrap();
        let public = PublicIdentity {
            account: AccountId::from_bytes([90; 16]),
            account_key: account_key.public_key(),
            device: DeviceId::from_bytes([92; 16]),
            device_key: device_key.public_key(),
            peer: b"managed-admission-test-peer".to_vec(),
        };
        let mut spec = support::strategic_world().to_spec();
        let template = spec.registry[0].clone();
        for index in 1..provider_count {
            let mut extra = template.clone();
            extra.id = ProviderId::from_bytes([u8::try_from(index).unwrap(); 16]);
            let aggregate = AggregateId::from_bytes([128 + u8::try_from(index).unwrap(); 16]);
            spec.providers.push(ProviderState {
                id: extra.id,
                aggregate,
                draws: 0,
                cooldown_until: WorldTick(0),
            });
            spec.revisions.insert(aggregate, AggregateRevision(0));
            spec.ownership.push(OwnershipRecord {
                aggregate,
                provider: extra.id,
                generation: 1,
                activation_seq: EventSeq(0),
                ruleset_hash: [4; 32],
            });
            spec.registry.push(extra);
        }
        let supported: Vec<_> = spec
            .registry
            .iter()
            .map(|p| ProviderVersion {
                provider: p.id,
                implementation: p.implementation_hash,
                version: p.version,
                state_schema: p.state_schema,
            })
            .collect();
        let from = Composition::new(1, SOURCE_SCHEMA, [2; 32], [3; 32], supported.clone()).unwrap();
        let to = Composition::new(4, TARGET_SCHEMA, [9; 32], [3; 32], supported.clone()).unwrap();
        let world = World::new(spec)
            .unwrap()
            .with_ruleset_hash(from.digest())
            .unwrap();
        let mut raw = Store::create(scratch.db(), &world).unwrap();
        let scope = raw.known_frontiers().unwrap().scope;
        raw.commit_membership(None, &MembershipState::bootstrap(scope, &public).unwrap())
            .unwrap();
        let authority = support::authority();
        let frontier = nf_kernel::admit(&world, vec![], authority).unwrap();
        let Settlement::Committed { batch, .. } =
            nf_kernel::settle(&world, &frontier, vec![], authority, MissingPolicy::Pause).unwrap()
        else {
            panic!("complete empty frontier");
        };
        raw.prepare(&frontier, &[], &[]).unwrap();
        raw.commit(&batch).unwrap();
        let known = raw.known_frontiers().unwrap();
        drop(raw);
        let binary_root = scratch.0.join("binaries");
        std::fs::create_dir(&binary_root).unwrap();
        let exe = std::env::current_exe().unwrap();
        let metadata = std::fs::metadata(&exe).unwrap();
        assert!(metadata.len() <= 64 * 1024 * 1024);
        let bytes = std::fs::read(&exe).unwrap();
        assert!(bytes.starts_with(b"MZ") || bytes.starts_with(b"\x7fELF"));
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let pin = |name: &str| {
            let path = binary_root.join(name);
            std::fs::write(&path, &bytes).unwrap();
            FilePin::new(
                path,
                u64::try_from(bytes.len()).unwrap(),
                digest,
                PROVENANCE,
            )
            .unwrap()
        };
        let old = pin("old-test-executable.bin");
        let new = pin("new-test-executable.bin");
        Self {
            scratch,
            known,
            public,
            account_key,
            device_key,
            from,
            to,
            supported,
            old,
            new,
            cutoff,
        }
    }
    pub fn catalog(&self) -> PathBuf {
        self.scratch.0.join("catalog.sqlite")
    }
    pub fn begin(&self, supported: &[ProviderVersion]) -> Result<AdoptionDraft, AdoptionFailure> {
        self.begin_at(self.scratch.db(), supported)
    }
    pub fn begin_at(
        &self,
        source_path: PathBuf,
        supported: &[ProviderVersion],
    ) -> Result<AdoptionDraft, AdoptionFailure> {
        let source = Store::open_existing(self.scratch.db(), self.known).unwrap();
        LocalCoordinator::begin_adoption(
            source,
            self.catalog(),
            AdoptionInput {
                source_path,
                minimum: self.known,
                composition: &self.from,
                supported,
                binary_evidence: BinaryEvidence::library_fixture(
                    self.old.clone(),
                    self.new.clone(),
                    &self.from,
                    &self.to,
                    PROVENANCE,
                ),
                artifact_provenance: PROVENANCE,
            },
        )
    }
    pub fn proof(&self, challenge: [u8; 32]) -> DeviceProof {
        let mut proof = DeviceProof {
            scope: self.known.scope,
            account: self.public.account,
            device: self.public.device,
            frontier: self.known.membership_revision.unwrap(),
            peer: self.public.peer.clone(),
            challenge,
            signature: [0; 64],
        };
        proof.signature = self.device_key.sign(&device_digest(&proof).unwrap());
        proof
    }
    pub fn adopt(&self) -> SourceOwner {
        let mut draft = self
            .begin(&self.supported)
            .expect("reopened ordinary source must enter adoption");
        let challenge = draft
            .issue_adoption_challenge(&self.public, self.cutoff)
            .unwrap();
        draft
            .finish_adoption(self.proof(challenge.digest()))
            .unwrap()
    }
    pub fn sealed(&self) -> ManagedDrainedSource {
        let mut owner = self.adopt();
        let policy = QuiescePolicy::RequireAlreadyDrained;
        let challenge = owner
            .issue_quiesce_challenge(policy, &self.public, self.cutoff)
            .unwrap();
        owner
            .quiesce_and_seal(policy, self.known, self.proof(challenge.digest()))
            .unwrap()
    }
}
