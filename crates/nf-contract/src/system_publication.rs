//! Bounded owned publication history. Records grant no owner, storage or game authority.
use crate::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_topology::{self, Limits, Topology},
};
use alloc::vec::Vec;
use core::fmt;

const MAX_KEYS: usize = 256;
const MAX_RECORDS: usize = 512;
const MAX_CANONICAL_BYTES: usize = 8_388_608;
type Key = (HistoryId, AccountId, EntityId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationProfile {
    SelectedTopologyV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationKind {
    PublishedVersion,
    Tombstone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicationLimits {
    pub keys: usize,
    pub records: usize,
    pub canonical_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationOutcome {
    PublicationAppended,
    TombstoneAppended,
    Unchanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationError {
    InvalidNamespace,
    Bounds,
    InvalidInput,
    UniverseMismatch,
    MissingPublication,
    StaleRevision,
    RevisionConflict,
    KeyLimit,
    RecordLimit,
    ByteLimit,
    Topology(system_topology::Error),
}

#[derive(Clone, Eq, PartialEq)]
pub struct PublicationRecord {
    universe: UniverseId,
    history: HistoryId,
    owner: AccountId,
    system: EntityId,
    profile: PublicationProfile,
    kind: PublicationKind,
    revision: u64,
    referenced_publication_revision: u64,
    referenced_topology_sha256: [u8; 32],
    canonical_topology_bytes: Option<Vec<u8>>,
}

impl PublicationRecord {
    pub fn universe(&self) -> UniverseId {
        self.universe
    }

    pub fn history(&self) -> HistoryId {
        self.history
    }

    pub fn owner(&self) -> AccountId {
        self.owner
    }

    pub fn system(&self) -> EntityId {
        self.system
    }

    pub fn profile(&self) -> PublicationProfile {
        self.profile
    }

    pub fn kind(&self) -> PublicationKind {
        self.kind
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn referenced_publication_revision(&self) -> u64 {
        self.referenced_publication_revision
    }

    pub fn referenced_topology_sha256(&self) -> [u8; 32] {
        self.referenced_topology_sha256
    }

    pub fn canonical_topology_bytes(&self) -> Option<&[u8]> {
        self.canonical_topology_bytes.as_deref()
    }

    fn key(&self) -> Key {
        (self.history, self.owner, self.system)
    }
}

impl fmt::Debug for PublicationRecord {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output
            .debug_struct("PublicationRecord")
            .field("profile", &self.profile)
            .field("kind", &self.kind)
            .field("revision", &self.revision)
            .field(
                "referenced_publication_revision",
                &self.referenced_publication_revision,
            )
            .field(
                "canonical_bytes",
                &self.canonical_topology_bytes.as_ref().map_or(0, Vec::len),
            )
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct PublicationHistory {
    universe: UniverseId,
    limits: PublicationLimits,
    records: Vec<PublicationRecord>,
    keys: usize,
    canonical_bytes: usize,
}

impl PublicationHistory {
    pub fn new(universe: UniverseId, limits: PublicationLimits) -> Result<Self, PublicationError> {
        if !nonzero(universe.as_bytes()) {
            return Err(PublicationError::InvalidNamespace);
        }
        if !(1..=MAX_KEYS).contains(&limits.keys)
            || !(1..=MAX_RECORDS).contains(&limits.records)
            || !(1..=MAX_CANONICAL_BYTES).contains(&limits.canonical_bytes)
        {
            return Err(PublicationError::Bounds);
        }
        Ok(Self {
            universe,
            limits,
            records: Vec::new(),
            keys: 0,
            canonical_bytes: 0,
        })
    }

    pub fn universe(&self) -> UniverseId {
        self.universe
    }

    pub fn limits(&self) -> PublicationLimits {
        self.limits
    }

    pub fn keys_len(&self) -> usize {
        self.keys
    }

    pub fn canonical_bytes_len(&self) -> usize {
        self.canonical_bytes
    }

    pub fn records(&self) -> &[PublicationRecord] {
        &self.records
    }

    pub fn latest(
        &self,
        history: HistoryId,
        owner: AccountId,
        system: EntityId,
    ) -> Option<&PublicationRecord> {
        let key = (history, owner, system);
        self.records.iter().rev().find(|record| record.key() == key)
    }

    pub fn append_publication(
        &mut self,
        topology: &Topology,
        topology_limits: Limits,
    ) -> Result<PublicationOutcome, PublicationError> {
        system_topology::validate(topology, topology_limits).map_err(PublicationError::Topology)?;
        if topology.universe != self.universe {
            return Err(PublicationError::UniverseMismatch);
        }
        let bytes = system_topology::encode(topology, topology_limits)
            .map_err(PublicationError::Topology)?;
        let sha256 = system_topology::digest(topology, topology_limits)
            .map_err(PublicationError::Topology)?;
        let key = (topology.history, topology.owner, topology.system);
        let index = match self.position(key, topology.revision) {
            Ok(index) => {
                let record = &self.records[index];
                return if record.kind == PublicationKind::PublishedVersion
                    && record.profile == PublicationProfile::SelectedTopologyV1
                    && record.canonical_topology_bytes.as_deref() == Some(bytes.as_slice())
                {
                    Ok(PublicationOutcome::Unchanged)
                } else {
                    Err(PublicationError::RevisionConflict)
                };
            }
            Err(index) => index,
        };
        let latest = self.latest(topology.history, topology.owner, topology.system);
        if latest.is_some_and(|record| topology.revision < record.revision) {
            return Err(PublicationError::StaleRevision);
        }
        let new_key = latest.is_none();
        let (keys, canonical_bytes) = self.preflight(new_key, bytes.len())?;
        let record = PublicationRecord {
            universe: self.universe,
            history: topology.history,
            owner: topology.owner,
            system: topology.system,
            profile: PublicationProfile::SelectedTopologyV1,
            kind: PublicationKind::PublishedVersion,
            revision: topology.revision,
            referenced_publication_revision: topology.revision,
            referenced_topology_sha256: sha256,
            canonical_topology_bytes: Some(bytes),
        };
        self.records.insert(index, record);
        self.keys = keys;
        self.canonical_bytes = canonical_bytes;
        Ok(PublicationOutcome::PublicationAppended)
    }

    pub fn append_tombstone(
        &mut self,
        history: HistoryId,
        owner: AccountId,
        system: EntityId,
        revision: u64,
    ) -> Result<PublicationOutcome, PublicationError> {
        if !nonzero(history.as_bytes())
            || !nonzero(owner.as_bytes())
            || !nonzero(system.as_bytes())
            || revision == 0
        {
            return Err(PublicationError::InvalidInput);
        }
        let key = (history, owner, system);
        let latest = self
            .latest(history, owner, system)
            .ok_or(PublicationError::MissingPublication)?;
        let latest_revision = latest.revision;
        let index = match self.position(key, revision) {
            Ok(index) => {
                return if self.records[index].kind == PublicationKind::Tombstone {
                    Ok(PublicationOutcome::Unchanged)
                } else {
                    Err(PublicationError::RevisionConflict)
                };
            }
            Err(index) => index,
        };
        if revision < latest_revision {
            return Err(PublicationError::StaleRevision);
        }
        let publication = self
            .records
            .iter()
            .rev()
            .find(|record| record.key() == key && record.kind == PublicationKind::PublishedVersion)
            .ok_or(PublicationError::MissingPublication)?;
        let referenced_publication_revision = publication.revision;
        let referenced_topology_sha256 = publication.referenced_topology_sha256;
        let (keys, canonical_bytes) = self.preflight(false, 0)?;
        let record = PublicationRecord {
            universe: self.universe,
            history,
            owner,
            system,
            profile: PublicationProfile::SelectedTopologyV1,
            kind: PublicationKind::Tombstone,
            revision,
            referenced_publication_revision,
            referenced_topology_sha256,
            canonical_topology_bytes: None,
        };
        self.records.insert(index, record);
        self.keys = keys;
        self.canonical_bytes = canonical_bytes;
        Ok(PublicationOutcome::TombstoneAppended)
    }

    fn position(&self, key: Key, revision: u64) -> Result<usize, usize> {
        self.records
            .binary_search_by_key(&(key, revision), |record| (record.key(), record.revision))
    }

    fn preflight(&self, new_key: bool, bytes: usize) -> Result<(usize, usize), PublicationError> {
        let keys = self
            .keys
            .checked_add(usize::from(new_key))
            .filter(|value| *value <= self.limits.keys)
            .ok_or(PublicationError::KeyLimit)?;
        let records = self
            .records
            .len()
            .checked_add(1)
            .ok_or(PublicationError::RecordLimit)?;
        if records > self.limits.records {
            return Err(PublicationError::RecordLimit);
        }
        let canonical_bytes = self
            .canonical_bytes
            .checked_add(bytes)
            .filter(|value| *value <= self.limits.canonical_bytes)
            .ok_or(PublicationError::ByteLimit)?;
        Ok((keys, canonical_bytes))
    }
}

impl fmt::Debug for PublicationHistory {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output
            .debug_struct("PublicationHistory")
            .field("limits", &self.limits)
            .field("keys", &self.keys)
            .field("records", &self.records.len())
            .field("canonical_bytes", &self.canonical_bytes)
            .finish()
    }
}

fn nonzero(bytes: &[u8; 16]) -> bool {
    bytes.iter().any(|byte| *byte != 0)
}
