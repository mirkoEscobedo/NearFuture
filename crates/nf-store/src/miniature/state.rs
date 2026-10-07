use super::{error::Result, model::*};
use crate::StoreError;
use nf_contract::identity::*;
use nf_identity::model::Scope;
use nf_kernel::miniature::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct State {
    pub world: MiniatureWorld,
    pub authority: Option<MiniatureAuthority>,
    pub pending: Option<MiniatureFrontier>,
    pub requests: BTreeMap<RequestId, MiniatureBoundRequest>,
    pub outbox: BTreeMap<OperationId, MiniatureOutbox>,
}
impl State {
    pub fn initial(world: MiniatureWorld) -> Self {
        Self {
            world,
            authority: None,
            pending: None,
            requests: BTreeMap::new(),
            outbox: BTreeMap::new(),
        }
    }
    pub fn scope(&self) -> Scope {
        let g = self.world.component().genesis();
        Scope {
            universe: g.universe,
            history: g.history,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.requests.len() > 4096 || self.outbox.len() > 256 {
            return Err(StoreError::Limit.into());
        }
        if self
            .authority
            .is_some_and(|a| a.term.0 == 0 || a.session.0 == 0)
        {
            return Err(StoreError::Corrupt.into());
        }
        let mut operations = BTreeSet::new();
        let mut jobs = BTreeSet::new();
        let mut pending = BTreeSet::new();
        for (id, record) in &self.requests {
            let i = &record.intent;
            if *id != i.request
                || i.universe != self.scope().universe
                || i.history != self.scope().history
                || i.provider != self.world.metadata().provider
                || !operations.insert(i.operation)
                || !jobs.insert(i.job)
                || miniature_request_binding(i)?.digest() != record.binding_digest
            {
                return Err(StoreError::Corrupt.into());
            }
            match record.status {
                MiniatureRequestStatus::Pending { operation } => {
                    if operation != i.operation {
                        return Err(StoreError::Corrupt.into());
                    }
                    pending.insert(*id);
                }
                MiniatureRequestStatus::Committed {
                    operation,
                    sequence,
                    rejection,
                } => {
                    if operation != i.operation
                        || sequence.0 == 0
                        || sequence > self.world.metadata().event_sequence
                        || !self.world.outcomes().iter().any(|o| {
                            o.operation == operation && o.job == i.job && o.rejection == rejection
                        })
                    {
                        return Err(StoreError::Corrupt.into());
                    }
                }
            }
        }
        if self.world.outcomes().len() != self.requests.len() - pending.len() {
            return Err(StoreError::Corrupt.into());
        }
        if let Some(f) = &self.pending {
            if self.authority.is_none()
                || f.snapshot() != &self.world
                || f.input_hash() != miniature_state_hash(&self.world)?
                || f.intents().len() != pending.len()
            {
                return Err(StoreError::Corrupt.into());
            }
            for i in f.intents() {
                if !pending.remove(&i.request) || self.requests[&i.request].intent != *i {
                    return Err(StoreError::Corrupt.into());
                }
            }
        }
        if !pending.is_empty() {
            return Err(StoreError::Corrupt.into());
        }
        for (id, o) in &self.outbox {
            if *id != o.operation
                || !self.requests.values().any(|r| {
                    r.status
                        == MiniatureRequestStatus::Committed {
                            operation: *id,
                            sequence: o.sequence,
                            rejection: o.rejection,
                        }
                })
            {
                return Err(StoreError::Corrupt.into());
            }
        }
        Ok(())
    }
}
