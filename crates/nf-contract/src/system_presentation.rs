//! Bounded display coordinates for real conceptual reservations, never native placement authority.
use crate::{
    identity::{AccountId, EntityId, HistoryId, UniverseId},
    system_placement::{ConceptualSlot, PlacementRegistry},
};
use core::fmt;

const GRID_SIDE: u32 = 256;
const MAX_DIMENSION: u32 = 16_777_216;
const MAX_HALF_EXTENT: u32 = 32_767;
const MAX_ORIGIN: i64 = 1_099_511_627_776;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationProfile {
    Grid256V1,
}

/// Integer quanta: 1024 quanta form one abstract display unit, not a game-space unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationDomain {
    pub origin_x: i64,
    pub origin_y: i64,
    pub width: u32,
    pub height: u32,
    pub half_extent: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationError {
    InvalidNamespace,
    Bounds,
    DoesNotFit,
    InvalidInput,
    UniverseMismatch,
    MissingPlacement,
}

/// The complete immutable namespace/domain/profile value is the display epoch.
#[derive(Clone, Eq, PartialEq)]
pub struct ClientPresentation {
    universe: UniverseId,
    domain: PresentationDomain,
}

impl ClientPresentation {
    pub fn new(
        universe: UniverseId,
        domain: PresentationDomain,
    ) -> Result<Self, PresentationError> {
        if !nonzero(universe.as_bytes()) {
            return Err(PresentationError::InvalidNamespace);
        }
        if !(-MAX_ORIGIN..=MAX_ORIGIN).contains(&domain.origin_x)
            || !(-MAX_ORIGIN..=MAX_ORIGIN).contains(&domain.origin_y)
            || !(1..=MAX_DIMENSION).contains(&domain.width)
            || !(1..=MAX_DIMENSION).contains(&domain.height)
            || domain.half_extent > MAX_HALF_EXTENT
        {
            return Err(PresentationError::Bounds);
        }
        let minimum_pitch = domain
            .half_extent
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or(PresentationError::Bounds)?;
        if domain.width / GRID_SIDE < minimum_pitch || domain.height / GRID_SIDE < minimum_pitch {
            return Err(PresentationError::DoesNotFit);
        }
        Ok(Self { universe, domain })
    }

    pub fn profile(&self) -> PresentationProfile {
        PresentationProfile::Grid256V1
    }

    pub fn universe(&self) -> UniverseId {
        self.universe
    }

    pub fn domain(&self) -> PresentationDomain {
        self.domain
    }

    /// Observe one current registry value; this does not admit content or materialization.
    pub fn project(
        &self,
        registry: &PlacementRegistry,
        history: HistoryId,
        owner: AccountId,
        system: EntityId,
    ) -> Result<ProjectedPlacement, PresentationError> {
        if !nonzero(history.as_bytes()) || !nonzero(owner.as_bytes()) || !nonzero(system.as_bytes())
        {
            return Err(PresentationError::InvalidInput);
        }
        if registry.universe() != self.universe {
            return Err(PresentationError::UniverseMismatch);
        }
        let entry = registry
            .entries()
            .iter()
            .find(|entry| {
                entry.history() == history && entry.owner() == owner && entry.system() == system
            })
            .ok_or(PresentationError::MissingPlacement)?;
        let slot = u64::from(entry.slot().value());
        let x = coordinate(self.domain.origin_x, self.domain.width, slot % 256)?;
        let y = coordinate(self.domain.origin_y, self.domain.height, slot / 256)?;
        Ok(ProjectedPlacement {
            universe: self.universe,
            history: entry.history(),
            owner: entry.owner(),
            system: entry.system(),
            slot: entry.slot(),
            revision: entry.revision(),
            topology_sha256: entry.topology_sha256(),
            domain: self.domain,
            x,
            y,
        })
    }
}

impl fmt::Debug for ClientPresentation {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("ClientPresentation { profile: Grid256V1 }")
    }
}

/// Owned source binding and display value, with no live registry borrow or authority grant.
#[derive(Clone, Eq, PartialEq)]
pub struct ProjectedPlacement {
    universe: UniverseId,
    history: HistoryId,
    owner: AccountId,
    system: EntityId,
    slot: ConceptualSlot,
    revision: u64,
    topology_sha256: [u8; 32],
    domain: PresentationDomain,
    x: i64,
    y: i64,
}

impl ProjectedPlacement {
    pub fn profile(&self) -> PresentationProfile {
        PresentationProfile::Grid256V1
    }

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

    pub fn slot(&self) -> ConceptualSlot {
        self.slot
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn topology_sha256(&self) -> [u8; 32] {
        self.topology_sha256
    }

    pub fn domain(&self) -> PresentationDomain {
        self.domain
    }

    pub fn x(&self) -> i64 {
        self.x
    }

    pub fn y(&self) -> i64 {
        self.y
    }
}

impl fmt::Debug for ProjectedPlacement {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("ProjectedPlacement { profile: Grid256V1 }")
    }
}

fn coordinate(origin: i64, span: u32, index: u64) -> Result<i64, PresentationError> {
    let pitch = u64::from(span) / u64::from(GRID_SIDE);
    let offset = index
        .checked_mul(pitch)
        .and_then(|value| value.checked_add(pitch / 2))
        .ok_or(PresentationError::Bounds)?;
    let offset = i64::try_from(offset).map_err(|_| PresentationError::Bounds)?;
    origin.checked_add(offset).ok_or(PresentationError::Bounds)
}

fn nonzero(value: &[u8; 16]) -> bool {
    value.iter().any(|byte| *byte != 0)
}
