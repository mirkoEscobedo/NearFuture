use crate::*;
use nf_contract::identity::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndustryKind {
    Farming = 1,
    Workshop = 2,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldAction {
    CreateColony {
        site: EntityId,
        faction: EntityId,
    },
    BuildIndustry {
        market: EntityId,
        kind: IndustryKind,
    },
    SetRelationship {
        faction: EntityId,
        other: EntityId,
        score: i32,
    },
    Travel {
        fleet: EntityId,
        destination: EntityId,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub request: RequestId,
    pub operation: OperationId,
    pub job: JobId,
    pub actor: AccountId,
    pub expected_revision: AggregateRevision,
    pub action: WorldAction,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceHold {
    pub(crate) operation: OperationId,
    pub(crate) faction: EntityId,
    pub(crate) credits: u64,
    pub(crate) supplies: u64,
}
impl ResourceHold {
    pub fn operation(&self) -> OperationId {
        self.operation
    }
    pub fn faction(&self) -> EntityId {
        self.faction
    }
    pub fn credits(&self) -> u64 {
        self.credits
    }
    pub fn supplies(&self) -> u64 {
        self.supplies
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPlan {
    pub(crate) source: State,
    pub(crate) candidate: Candidate,
    pub(crate) commit_tick: WorldTick,
    pub(crate) hold: ResourceHold,
}
impl ActionPlan {
    pub fn reservation(&self) -> &ResourceHold {
        &self.hold
    }
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }
    pub fn committing_tick(&self) -> WorldTick {
        self.commit_tick
    }
}
pub fn evaluate(
    state: &State,
    commit_tick: WorldTick,
    candidate: &Candidate,
) -> Result<ActionPlan, WorldError> {
    if candidate.expected_revision != state.revision {
        return Err(WorldError::StaleRevision);
    }
    if commit_tick.0 == 0 {
        return Err(WorldError::InvalidValue);
    }
    validate_at(state, WorldTick(commit_tick.0 - 1))?;
    state.revision.checked_next().ok_or(WorldError::Overflow)?;
    let (faction, credits, supplies) = match candidate.action {
        WorldAction::CreateColony { site, faction } => {
            let f = state
                .factions
                .iter()
                .find(|f| f.id == faction)
                .ok_or(WorldError::InvalidReference)?;
            let m = state
                .markets
                .iter()
                .find(|m| m.id == site)
                .ok_or(WorldError::InvalidReference)?;
            if m.owner.is_some() {
                return Err(WorldError::Locked);
            }
            if m.system != crate::genesis::entity(state.genesis, 1, u32::from(f.ordinal))
                || !state
                    .fleets
                    .iter()
                    .any(|v| v.faction == f.id && v.location == FleetLocation::Docked(m.system))
            {
                return Err(WorldError::InvalidReference);
            }
            (faction, 100, 40)
        }
        WorldAction::BuildIndustry { market, kind } => {
            let m = state
                .markets
                .iter()
                .find(|m| m.id == market)
                .ok_or(WorldError::InvalidReference)?;
            let faction = m.owner.ok_or(WorldError::InvalidReference)?;
            if m.industries[kind as usize - 1] != IndustryState::Absent {
                return Err(WorldError::Locked);
            }
            if state.schedules.len() >= 32 {
                return Err(WorldError::Limit);
            }
            commit_tick.0.checked_add(2).ok_or(WorldError::Overflow)?;
            (faction, 60, 20)
        }
        WorldAction::Travel { fleet, destination } => {
            let fleet = state
                .fleets
                .iter()
                .find(|v| v.id == fleet)
                .ok_or(WorldError::InvalidReference)?;
            let FleetLocation::Docked(origin) = fleet.location else {
                return Err(WorldError::Locked);
            };
            if origin == destination || !state.systems.iter().any(|s| s.id == destination) {
                return Err(WorldError::InvalidReference);
            }
            if state.schedules.len() >= 32 {
                return Err(WorldError::Limit);
            }
            commit_tick.0.checked_add(3).ok_or(WorldError::Overflow)?;
            (fleet.faction, 0, 20)
        }
        WorldAction::SetRelationship {
            faction,
            other,
            score,
        } => {
            if !(-10000..=10000).contains(&score) {
                return Err(WorldError::InvalidValue);
            }
            if faction == other
                || !state
                    .relations
                    .iter()
                    .any(|r| r.left == faction.min(other) && r.right == faction.max(other))
            {
                return Err(WorldError::InvalidReference);
            }
            (faction, 0, 0)
        }
    };
    let f = state
        .factions
        .iter()
        .find(|f| f.id == faction)
        .ok_or(WorldError::InvalidReference)?;
    if f.account != candidate.actor {
        return Err(WorldError::Unauthorized);
    }
    if f.credits < credits || f.supplies < supplies {
        return Err(WorldError::Resources);
    }
    Ok(ActionPlan {
        source: state.clone(),
        candidate: candidate.clone(),
        commit_tick,
        hold: ResourceHold {
            operation: candidate.operation,
            faction,
            credits,
            supplies,
        },
    })
}
pub fn apply_plan(
    state: &State,
    commit_tick: WorldTick,
    plan: &ActionPlan,
) -> Result<State, WorldError> {
    if state != &plan.source || commit_tick != plan.commit_tick {
        return Err(WorldError::StaleRevision);
    }
    let mut next = state.clone();
    let f = next
        .factions
        .iter_mut()
        .find(|f| f.id == plan.hold.faction)
        .ok_or(WorldError::InvalidReference)?;
    f.credits = f
        .credits
        .checked_sub(plan.hold.credits)
        .ok_or(WorldError::Resources)?;
    f.supplies = f
        .supplies
        .checked_sub(plan.hold.supplies)
        .ok_or(WorldError::Resources)?;
    f.spent_credits = f
        .spent_credits
        .checked_add(plan.hold.credits)
        .ok_or(WorldError::Overflow)?;
    f.spent_supplies = f
        .spent_supplies
        .checked_add(plan.hold.supplies)
        .ok_or(WorldError::Overflow)?;
    match plan.candidate.action {
        WorldAction::CreateColony { site, faction } => {
            next.markets
                .iter_mut()
                .find(|m| m.id == site)
                .ok_or(WorldError::InvalidReference)?
                .owner = Some(faction)
        }
        WorldAction::BuildIndustry { market, kind } => {
            let id =
                crate::schedules::schedule_id(next.genesis, plan.candidate.operation, 1, market);
            if next.schedules.iter().any(|s| s.id == id) {
                return Err(WorldError::Duplicate);
            }
            next.markets
                .iter_mut()
                .find(|m| m.id == market)
                .ok_or(WorldError::InvalidReference)?
                .industries[kind as usize - 1] = IndustryState::Constructing(id);
            next.schedules.push(Schedule {
                id,
                operation: plan.candidate.operation,
                due: WorldTick(
                    plan.commit_tick
                        .0
                        .checked_add(2)
                        .ok_or(WorldError::Overflow)?,
                ),
                kind: ScheduleKind::Industry {
                    market,
                    industry: kind,
                },
            });
            next.schedules.sort_by_key(|s| (s.due, s.id));
        }
        WorldAction::Travel { fleet, destination } => {
            let id =
                crate::schedules::schedule_id(next.genesis, plan.candidate.operation, 2, fleet);
            if next.schedules.iter().any(|s| s.id == id) {
                return Err(WorldError::Duplicate);
            }
            let f = next
                .fleets
                .iter_mut()
                .find(|v| v.id == fleet)
                .ok_or(WorldError::InvalidReference)?;
            let FleetLocation::Docked(origin) = f.location else {
                return Err(WorldError::Locked);
            };
            f.location = FleetLocation::InTransit(id);
            next.schedules.push(Schedule {
                id,
                operation: plan.candidate.operation,
                due: WorldTick(
                    plan.commit_tick
                        .0
                        .checked_add(3)
                        .ok_or(WorldError::Overflow)?,
                ),
                kind: ScheduleKind::Arrival {
                    fleet,
                    origin,
                    destination,
                },
            });
            next.schedules.sort_by_key(|s| (s.due, s.id));
        }
        WorldAction::SetRelationship {
            faction,
            other,
            score,
        } => {
            next.relations
                .iter_mut()
                .find(|r| r.left == faction.min(other) && r.right == faction.max(other))
                .ok_or(WorldError::InvalidReference)?
                .score = score
        }
    }
    next.revision = next.revision.checked_next().ok_or(WorldError::Overflow)?;
    crate::validation::validate_action_output(&next, commit_tick)?;
    Ok(next)
}
