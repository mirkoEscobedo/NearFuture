use super::count;
use crate::{error::Result, model::{RequestStatus, State}};
use nf_kernel::WorldSpec;

#[derive(Clone, Debug)]
pub struct StrategicSupportCountsV1 {
    factions: u64,
    relations: u64,
    markets: u64,
    providers: u64,
    registry: u64,
    assignments: u64,
    pending: u64,
    pending_requests: u64,
    committed_requests: u64,
    reservations: u64,
    outbox: u64,
    members: Option<u64>,
}

impl StrategicSupportCountsV1 {
    pub(super) fn from_recovered(
        state: &State,
        spec: &WorldSpec,
        members: Option<u64>,
    ) -> Result<Self> {
        let pending_requests = state.requests.values().filter(|record| {
            matches!(record.status, RequestStatus::Pending { .. })
        }).count();
        let committed_requests = state.requests.values().filter(|record| {
            matches!(record.status, RequestStatus::Committed { .. })
        }).count();
        Ok(Self {
            factions: count(spec.factions.len())?,
            relations: count(spec.relations.len())?,
            markets: count(spec.markets.len())?,
            providers: count(spec.providers.len())?,
            registry: count(spec.registry.len())?,
            assignments: count(spec.ownership.len())?,
            pending: u64::from(state.pending.is_some()),
            pending_requests: count(pending_requests)?,
            committed_requests: count(committed_requests)?,
            reservations: count(state.reservations.len())?,
            outbox: count(state.outbox.len())?,
            members,
        })
    }
    pub fn factions(&self) -> u64 {
        self.factions
    }
    pub fn relations(&self) -> u64 {
        self.relations
    }
    pub fn markets(&self) -> u64 {
        self.markets
    }
    pub fn providers(&self) -> u64 {
        self.providers
    }
    pub fn registry(&self) -> u64 {
        self.registry
    }
    pub fn assignments(&self) -> u64 {
        self.assignments
    }
    pub fn pending(&self) -> u64 {
        self.pending
    }
    pub fn pending_requests(&self) -> u64 {
        self.pending_requests
    }
    pub fn committed_requests(&self) -> u64 {
        self.committed_requests
    }
    pub fn reservations(&self) -> u64 {
        self.reservations
    }
    pub fn outbox(&self) -> u64 {
        self.outbox
    }
    pub fn members(&self) -> Option<u64> {
        self.members
    }
}
