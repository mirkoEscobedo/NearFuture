use crate::error::{Result, StoreError};
use nf_contract::{canonical::binding::RequestBinding, identity::*};
use nf_identity::model::Scope;
use nf_kernel::{Command, Frontier, Intent, Rejection, World};
use std::collections::BTreeMap;
pub const MAX_STATE_BYTES: usize = 1_048_576;
pub const MAX_REQUESTS: usize = 4096;
pub const MAX_RESERVATIONS: usize = 64;
pub const MAX_OUTBOX: usize = 256;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownFrontiers {
    pub scope: Scope,
    pub event_sequence: EventSeq,
    pub store_revision: u64,
    pub membership_revision: Option<u64>,
}
impl KnownFrontiers {
    pub fn genesis(universe: UniverseId, history: HistoryId) -> Self {
        Self {
            scope: Scope { universe, history },
            event_sequence: EventSeq(0),
            store_revision: 0,
            membership_revision: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrincipalDevice {
    pub request: RequestId,
    pub device: DeviceId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reservation {
    pub operation: OperationId,
    pub market: EntityId,
    pub amount: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestStatus {
    Pending {
        operation: OperationId,
    },
    Committed {
        operation: OperationId,
        sequence: EventSeq,
        rejection: Option<Rejection>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundRequestStatus {
    pub status: RequestStatus,
    pub binding_digest: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RequestRecord {
    pub intent: Intent,
    pub device: DeviceId,
    pub status: RequestStatus,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutboxRecord {
    pub operation: OperationId,
    pub sequence: EventSeq,
    pub batch_digest: [u8; 32],
    pub rejection: Option<Rejection>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct State {
    pub world: World,
    pub pending: Option<Frontier>,
    pub requests: BTreeMap<RequestId, RequestRecord>,
    pub reservations: BTreeMap<OperationId, Reservation>,
    pub outbox: BTreeMap<OperationId, OutboxRecord>,
}
impl State {
    pub fn initial(world: &World) -> Self {
        Self {
            world: world.clone(),
            pending: None,
            requests: BTreeMap::new(),
            reservations: BTreeMap::new(),
            outbox: BTreeMap::new(),
        }
    }
    pub fn scope(&self) -> Scope {
        let spec = self.world.to_spec();
        Scope {
            universe: spec.universe,
            history: spec.history,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.requests.len() > MAX_REQUESTS
            || self.reservations.len() > MAX_RESERVATIONS
            || self.outbox.len() > MAX_OUTBOX
        {
            return Err(StoreError::Limit);
        }
        let scope = self.scope();
        let pending: Vec<_> = self
            .requests
            .iter()
            .filter(|(_, r)| matches!(r.status, RequestStatus::Pending { .. }))
            .collect();
        if let Some(frontier) = &self.pending {
            if frontier.snapshot_world() != &self.world
                || frontier.input_hash() != nf_kernel::state_hash(&self.world)?
                || frontier.jobs().len() != pending.len()
            {
                return Err(StoreError::InvalidTransition);
            }
            for job in frontier.jobs() {
                let record = self
                    .requests
                    .get(&job.intent().request)
                    .ok_or(StoreError::InvalidTransition)?;
                if record.intent != *job.intent()
                    || !matches!(record.status, RequestStatus::Pending { .. })
                {
                    return Err(StoreError::InvalidTransition);
                }
            }
        } else if !pending.is_empty() || !self.reservations.is_empty() {
            return Err(StoreError::InvalidTransition);
        }
        for (request, record) in &self.requests {
            if *request != record.intent.request
                || record.intent.universe != scope.universe
                || record.intent.history != scope.history
            {
                return Err(StoreError::Scope);
            }
            match record.status {
                RequestStatus::Pending { operation } if operation == record.intent.operation => {}
                RequestStatus::Committed {
                    operation,
                    sequence,
                    rejection,
                } if operation == record.intent.operation
                    && sequence <= self.world.view().event_seq() =>
                {
                    if !self.world.outcomes().iter().any(|outcome| {
                        outcome.operation == operation
                            && outcome.job == record.intent.job
                            && outcome.rejection == rejection
                    }) {
                        return Err(StoreError::InvalidTransition);
                    }
                }
                _ => return Err(StoreError::InvalidTransition),
            }
        }
        crate::transitions::validate_reservations(self)?;
        for (operation, outbox) in &self.outbox {
            if *operation != outbox.operation
                || !self.requests.values().any(|r| {
                    r.status
                        == RequestStatus::Committed {
                            operation: *operation,
                            sequence: outbox.sequence,
                            rejection: outbox.rejection,
                        }
                })
            {
                return Err(StoreError::InvalidTransition);
            }
        }
        Ok(())
    }
}
pub(crate) fn binding(intent: &Intent, device: DeviceId) -> Result<RequestBinding> {
    let kind = match intent.command {
        Command::SeekPeace { .. } => 1,
        Command::AdjustRelation { .. } => 2,
        Command::AdjustMarket { .. } => 3,
    };
    Ok(RequestBinding {
        request_id: intent.request,
        account_id: intent.actor,
        device_id: device,
        universe_id: intent.universe,
        history_id: intent.history,
        operation_kind: kind,
        payload_digest: nf_kernel::intent_digest(intent)?,
    })
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    BeforeTransaction,
    AfterWrites,
    BeforeCommit,
    AfterCommit,
    BeforeAcknowledgement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Accepted {
    revision: u64,
}
impl Accepted {
    pub(crate) fn new(revision: u64) -> Self {
        Self { revision }
    }
    pub fn revision(self) -> u64 {
        self.revision
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurableAck {
    pub(crate) revision: u64,
    pub(crate) sequence: EventSeq,
    pub(crate) state_hash: [u8; 32],
}
impl DurableAck {
    pub fn revision(self) -> u64 {
        self.revision
    }
    pub fn sequence(self) -> EventSeq {
        self.sequence
    }
    pub fn state_hash(self) -> [u8; 32] {
        self.state_hash
    }
}
/// A future consensus driver must supply an actual quorum commit decision; local acknowledgement is not one.
pub trait QuorumCommitPort {
    type Decision;
    fn await_commit(&mut self, entry_digest: [u8; 32]) -> Result<Self::Decision>;
}
