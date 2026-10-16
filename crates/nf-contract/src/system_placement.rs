//! Bounded conceptual slot reservations. These values grant no publication or game authority.
use crate::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_topology::{self, Limits, Topology},
};
use alloc::vec::Vec;
use core::cmp::Ordering;

const MAX_REGISTRY_ROWS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ConceptualSlot(u16);

impl ConceptualSlot {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(&self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementEntry {
    history: HistoryId,
    owner: AccountId,
    system: EntityId,
    slot: ConceptualSlot,
    revision: u64,
    topology_sha256: [u8; 32],
}

impl PlacementEntry {
    pub fn history(&self) -> HistoryId {
        self.history
    }

    pub fn owner(&self) -> AccountId {
        self.owner
    }

    pub fn system(&self) -> EntityId {
        self.system
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn topology_sha256(&self) -> [u8; 32] {
        self.topology_sha256
    }

    pub fn slot(&self) -> ConceptualSlot {
        self.slot
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReservationOutcome {
    Inserted,
    Unchanged,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementError {
    InvalidNamespace,
    Bounds,
    UniverseMismatch,
    SlotOccupied,
    SlotMismatch,
    StaleRevision,
    RevisionConflict,
    Topology(system_topology::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementRegistry {
    universe: UniverseId,
    capacity: usize,
    entries: Vec<PlacementEntry>,
}

impl PlacementRegistry {
    pub fn new(universe: UniverseId, capacity: usize) -> Result<Self, PlacementError> {
        if universe.as_bytes().iter().all(|byte| *byte == 0) {
            return Err(PlacementError::InvalidNamespace);
        }
        if !(1..=MAX_REGISTRY_ROWS).contains(&capacity) {
            return Err(PlacementError::Bounds);
        }
        Ok(Self {
            universe,
            capacity,
            entries: Vec::with_capacity(capacity),
        })
    }

    pub fn universe(&self) -> UniverseId {
        self.universe
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn entries(&self) -> &[PlacementEntry] {
        &self.entries
    }

    pub fn reserve(
        &mut self,
        topology: &Topology,
        slot: ConceptualSlot,
        limits: Limits,
    ) -> Result<ReservationOutcome, PlacementError> {
        system_topology::validate(topology, limits).map_err(PlacementError::Topology)?;
        if topology.universe != self.universe {
            return Err(PlacementError::UniverseMismatch);
        }
        let topology_sha256 =
            system_topology::digest(topology, limits).map_err(PlacementError::Topology)?;
        let key = (topology.history, topology.owner, topology.system);
        match self
            .entries
            .binary_search_by_key(&key, |entry| (entry.history, entry.owner, entry.system))
        {
            Ok(index) => {
                let entry = &mut self.entries[index];
                if slot != entry.slot {
                    return Err(PlacementError::SlotMismatch);
                }
                match topology.revision.cmp(&entry.revision) {
                    Ordering::Less => Err(PlacementError::StaleRevision),
                    Ordering::Equal => {
                        if topology_sha256 == entry.topology_sha256 {
                            Ok(ReservationOutcome::Unchanged)
                        } else {
                            Err(PlacementError::RevisionConflict)
                        }
                    }
                    Ordering::Greater => {
                        entry.revision = topology.revision;
                        entry.topology_sha256 = topology_sha256;
                        Ok(ReservationOutcome::Updated)
                    }
                }
            }
            Err(index) => {
                if self.entries.iter().any(|entry| entry.slot == slot) {
                    return Err(PlacementError::SlotOccupied);
                }
                if self.entries.len() == self.capacity {
                    return Err(PlacementError::Bounds);
                }
                self.entries.insert(
                    index,
                    PlacementEntry {
                        history: topology.history,
                        owner: topology.owner,
                        system: topology.system,
                        slot,
                        revision: topology.revision,
                        topology_sha256,
                    },
                );
                Ok(ReservationOutcome::Inserted)
            }
        }
    }
}
