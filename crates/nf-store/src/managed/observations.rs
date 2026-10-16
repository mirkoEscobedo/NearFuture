use super::{
    cancellation::AdministrativeCancellation,
    engine::Engine,
    error::{ManagedError, Result},
    foot::StorageFoot,
    stamp::{CatalogFrontier, OwnerStamp},
};
use crate::{KnownFrontiers, RequestStatus};
use nf_contract::identity::{AccountId, DeviceId, EventSeq, OperationId, RequestId};
use nf_identity::model::{Roles, Scope};
use nf_kernel::Rejection;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedRequestStatus {
    pub status: RequestStatus,
    pub binding_digest: [u8; 32],
    pub administrative_cancellation: Option<AdministrativeCancellation>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedOutboxRecord {
    pub operation: OperationId,
    pub sequence: EventSeq,
    pub batch_digest: [u8; 32],
    pub rejection: Option<Rejection>,
    pub administrative_cancellation: Option<AdministrativeCancellation>,
}
#[derive(Clone)]
pub struct ManagedSnapshot {
    core: Vec<u8>,
    envelope: Vec<u8>,
    foot: StorageFoot,
    stamp: OwnerStamp,
    frontier: CatalogFrontier,
}
impl ManagedSnapshot {
    pub fn core_bytes(&self) -> &[u8] {
        &self.core
    }
    pub fn envelope_bytes(&self) -> &[u8] {
        &self.envelope
    }
    pub fn known_frontiers(&self) -> KnownFrontiers {
        self.foot.known
    }
    pub fn storage_foot(&self) -> &StorageFoot {
        &self.foot
    }
    pub fn owner_stamp(&self) -> &OwnerStamp {
        &self.stamp
    }
    pub fn catalog_frontier(&self) -> CatalogFrontier {
        self.frontier
    }
}
pub(super) fn snapshot(engine: &mut Engine) -> Result<ManagedSnapshot> {
    engine.gate(false)?;
    let world = engine.world()?;
    Ok(ManagedSnapshot {
        core: world.core_bytes.clone(),
        envelope: world.envelope_bytes.clone(),
        foot: world.foot.clone(),
        stamp: engine.owned_stamp()?.clone(),
        frontier: engine.catalog.frontier,
    })
}
pub(super) fn query(
    engine: &mut Engine,
    request: RequestId,
    account: AccountId,
    device: DeviceId,
    scope: Scope,
) -> Result<Option<ManagedRequestStatus>> {
    engine.gate(false)?;
    let world = engine.world()?;
    if scope != world.state.core.scope() {
        return Err(ManagedError::Scope);
    }
    let member = engine.member()?;
    let caller = engine.owned_stamp()?;
    let candidate = member
        .devices
        .get(&device)
        .ok_or(ManagedError::Unauthorized)?;
    if candidate.account != account
        || candidate.revoked
        || !member.accounts.contains_key(&account)
        || ((caller.account, caller.device) != (account, device)
            && !member
                .accounts
                .get(&caller.account)
                .is_some_and(|a| a.roles.contains(Roles::MODERATOR)))
    {
        return Err(ManagedError::Unauthorized);
    }
    let Some(record) = world.state.core.requests.get(&request) else {
        return Ok(None);
    };
    if record.intent.actor != account || record.device != device {
        return Err(ManagedError::Unauthorized);
    }
    Ok(Some(ManagedRequestStatus {
        status: record.status,
        binding_digest: crate::model::binding(&record.intent, record.device)?.digest(),
        administrative_cancellation: world.state.cancellations.get(&request).cloned(),
    }))
}
/// Previously verified data only; never a fresh admission or health witness.
pub(super) struct OwnerData {
    pub state: super::state::FullState,
    pub foot: StorageFoot,
    pub core: Vec<u8>,
    pub envelope: Vec<u8>,
    pub stamp: OwnerStamp,
    pub frontier: CatalogFrontier,
}
impl OwnerData {
    pub fn from_engine(engine: &Engine) -> Result<Self> {
        let world = engine.world()?;
        Ok(Self {
            state: world.state.clone(),
            foot: world.foot.clone(),
            core: world.core_bytes.clone(),
            envelope: world.envelope_bytes.clone(),
            stamp: engine.owned_stamp()?.clone(),
            frontier: engine.catalog.frontier,
        })
    }
    pub fn refresh(&mut self, engine: &Engine) -> Result<()> {
        if !engine.catalog.quarantined
            && engine.catalog.state.pending.is_none()
            && matches!(engine.catalog.state.phase, 2 | 7)
            && engine.stamp.is_some()
        {
            *self = Self::from_engine(engine)?;
        }
        Ok(())
    }
}
pub(super) fn outbox(data: &OwnerData) -> impl Iterator<Item = ManagedOutboxRecord> + '_ {
    data.state.core.outbox.values().map(move |record| {
        let annotation = data
            .state
            .cancellations
            .values()
            .find(|c| c.operation == record.operation)
            .cloned();
        ManagedOutboxRecord {
            operation: record.operation,
            sequence: record.sequence,
            batch_digest: record.batch_digest,
            rejection: record.rejection,
            administrative_cancellation: annotation,
        }
    })
}
