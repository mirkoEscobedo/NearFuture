use crate::*;
use alloc::vec::Vec;
use nf_contract::identity::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedOutcome {
    pub request: RequestId,
    pub operation: OperationId,
    pub job: JobId,
    pub rejection: Option<WorldError>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierPlan {
    pub(crate) winner: Option<ActionPlan>,
    pub(crate) outcomes: Vec<PlannedOutcome>,
}
impl FrontierPlan {
    pub fn winner(&self) -> Option<&ActionPlan> {
        self.winner.as_ref()
    }
    pub fn outcomes(&self) -> &[PlannedOutcome] {
        &self.outcomes
    }
}
pub fn plan_reservations(
    state: &State,
    commit_tick: WorldTick,
    candidates: &[Candidate],
) -> Result<FrontierPlan, WorldError> {
    if candidates.len() > 64 {
        return Err(WorldError::Limit);
    }
    if commit_tick.0 == 0 {
        return Err(WorldError::InvalidValue);
    }
    validate_at(state, WorldTick(commit_tick.0 - 1))?;
    let mut requests = alloc::collections::BTreeSet::new();
    let mut operations = alloc::collections::BTreeSet::new();
    let mut jobs = alloc::collections::BTreeSet::new();
    for c in candidates {
        if !requests.insert(c.request) || !operations.insert(c.operation) || !jobs.insert(c.job) {
            return Err(WorldError::Duplicate);
        }
    }
    let mut order: Vec<_> = candidates.iter().collect();
    order.sort_by_key(|c| (action_target(state, &c.action), c.operation, c.job));
    let mut winner = None;
    let mut outcomes = Vec::new();
    for c in order {
        let rejection = match evaluate(state, commit_tick, c) {
            Err(e) => Some(e),
            Ok(plan) => {
                if winner.is_some() {
                    Some(WorldError::Conflict)
                } else {
                    winner = Some(plan);
                    None
                }
            }
        };
        outcomes.push(PlannedOutcome {
            request: c.request,
            operation: c.operation,
            job: c.job,
            rejection,
        });
    }
    Ok(FrontierPlan { winner, outcomes })
}
pub fn action_target(state: &State, action: &WorldAction) -> EntityId {
    match action {
        WorldAction::CreateColony { site, .. } => *site,
        WorldAction::BuildIndustry { market, .. } => *market,
        WorldAction::SetRelationship { faction, other, .. } => state
            .relations
            .iter()
            .find(|r| r.left == (*faction).min(*other) && r.right == (*faction).max(*other))
            .map_or(*faction, |r| r.id),
        WorldAction::Travel { fleet, .. } => *fleet,
    }
}
