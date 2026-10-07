use crate::*;
use alloc::collections::BTreeSet;
use nf_contract::identity::*;
use sha2::{Digest, Sha256};
pub const RULESET_CONFIG:&[u8]=b"NF-MINI-1\0systems=3;factions=3;markets=6;fleets=3;home=even;routes=triangle;credits=200;supplies=100;colony=100,40;industry=60,20,2;travel=0,20,3;relation=-10000,10000;frontier=64;schedules=32;slots=2;winner=1;hour=1;seedrng=none";
pub fn ruleset_hash() -> [u8; 32] {
    Sha256::digest(RULESET_CONFIG).into()
}
/// Validate a final snapshot against its trusted enclosing committed tick.
pub fn validate_at(s: &State, tick: WorldTick) -> Result<(), WorldError> {
    validate_phase(s, tick, false)
}
/// Only action-output permits existing timers due now, before exact due reduction.
pub(crate) fn validate_action_output(s: &State, tick: WorldTick) -> Result<(), WorldError> {
    validate_phase(s, tick, true)
}
fn validate_phase(s: &State, tick: WorldTick, allow_due_now: bool) -> Result<(), WorldError> {
    if s.systems.len() != 3
        || s.factions.len() != 3
        || s.markets.len() != 6
        || s.fleets.len() != 3
        || s.relations.len() != 3
        || s.schedules.len() > 32
    {
        return Err(WorldError::Limit);
    }
    let g = s.genesis;
    if BTreeSet::from(g.accounts).len() != 3 {
        return Err(WorldError::Duplicate);
    }
    if !sorted(s.systems.iter().map(|v| v.id))
        || !sorted(s.factions.iter().map(|v| v.id))
        || !sorted(s.markets.iter().map(|v| v.id))
        || !sorted(s.fleets.iter().map(|v| v.id))
        || !sorted(s.relations.iter().map(|v| v.id))
        || s.schedules
            .windows(2)
            .any(|w| (w[0].due, w[0].id) >= (w[1].due, w[1].id))
    {
        return Err(WorldError::Malformed);
    }
    let mut ids = BTreeSet::new();
    for id in s
        .systems
        .iter()
        .map(|v| v.id)
        .chain(s.factions.iter().map(|v| v.id))
        .chain(s.markets.iter().map(|v| v.id))
        .chain(s.fleets.iter().map(|v| v.id))
        .chain(s.relations.iter().map(|v| v.id))
        .chain(s.schedules.iter().map(|v| v.id))
    {
        if !ids.insert(id) {
            return Err(WorldError::Duplicate);
        }
    }
    let system = |id| s.systems.iter().any(|v| v.id == id);
    for v in &s.systems {
        if v.ordinal >= 3 || v.id != crate::genesis::entity(g, 1, v.ordinal.into()) {
            return Err(WorldError::InvalidReference);
        }
    }
    for v in &s.factions {
        if v.ordinal >= 3
            || v.id != crate::genesis::entity(g, 2, v.ordinal.into())
            || v.account != g.accounts[v.ordinal as usize]
        {
            return Err(WorldError::InvalidReference);
        }
        if v.credits.checked_add(v.spent_credits) != Some(200)
            || v.supplies.checked_add(v.spent_supplies) != Some(100)
        {
            return Err(WorldError::Resources);
        }
    }
    for m in &s.markets {
        if m.ordinal >= 6
            || m.id != crate::genesis::entity(g, 3, m.ordinal.into())
            || m.system != crate::genesis::entity(g, 1, u32::from(m.ordinal / 2))
        {
            return Err(WorldError::InvalidReference);
        }
        let home = crate::genesis::entity(g, 2, u32::from(m.ordinal / 2));
        if m.owner.is_some_and(|id| id != home)
            || (m.ordinal % 2 == 0 && m.owner != Some(home))
            || (m.owner.is_none() && m.industries != [IndustryState::Absent; 2])
        {
            return Err(WorldError::InvalidReference);
        }
        for (i, slot) in m.industries.iter().enumerate() {
            let IndustryState::Constructing(id) = slot else {
                continue;
            };
            if !s.schedules.iter().any(|event| {
                event.id == *id
                    && event.kind
                        == ScheduleKind::Industry {
                            market: m.id,
                            industry: if i == 0 {
                                IndustryKind::Farming
                            } else {
                                IndustryKind::Workshop
                            },
                        }
            }) {
                return Err(WorldError::InvalidReference);
            }
        }
    }
    validate_resource_costs(s)?;
    for (i, (a, b)) in [(0, 1), (0, 2), (1, 2)].into_iter().enumerate() {
        let x = crate::genesis::entity(g, 2, a);
        let y = crate::genesis::entity(g, 2, b);
        let id = crate::genesis::entity(g, 5, i as u32);
        let r = s
            .relations
            .iter()
            .find(|r| r.id == id)
            .ok_or(WorldError::InvalidReference)?;
        if r.left != x.min(y) || r.right != x.max(y) || !(-10000..=10000).contains(&r.score) {
            return Err(WorldError::InvalidValue);
        }
    }
    for ordinal in 0..3 {
        let f = s
            .fleets
            .iter()
            .find(|v| v.id == crate::genesis::entity(g, 4, ordinal))
            .ok_or(WorldError::InvalidReference)?;
        if f.faction != crate::genesis::entity(g, 2, ordinal) {
            return Err(WorldError::InvalidReference);
        }
        match f.location {
            FleetLocation::Docked(id) => {
                if !system(id) {
                    return Err(WorldError::InvalidReference);
                }
            }
            FleetLocation::InTransit(id) => {
                if !s.schedules.iter().any(|e| {
                    e.id == id && matches!(e.kind,ScheduleKind::Arrival{fleet,..}if fleet==f.id)
                }) {
                    return Err(WorldError::InvalidReference);
                }
            }
        }
    }
    let mut operations = BTreeSet::new();
    for e in &s.schedules {
        let remaining = e
            .due
            .0
            .checked_sub(tick.0)
            .ok_or(WorldError::InvalidValue)?;
        let maximum = match e.kind {
            ScheduleKind::Industry { .. } => 2,
            ScheduleKind::Arrival { .. } => 3,
        };
        if (remaining == 0 && !allow_due_now)
            || remaining > maximum
            || !operations.insert(e.operation)
        {
            return Err(WorldError::InvalidValue);
        }
        let (kind, subject) = match e.kind {
            ScheduleKind::Industry { market, industry } => {
                let m = s
                    .markets
                    .iter()
                    .find(|m| m.id == market)
                    .ok_or(WorldError::InvalidReference)?;
                if m.owner.is_none()
                    || m.industries[industry as usize - 1] != IndustryState::Constructing(e.id)
                {
                    return Err(WorldError::InvalidReference);
                }
                (1, market)
            }
            ScheduleKind::Arrival {
                fleet,
                origin,
                destination,
            } => {
                let f = s
                    .fleets
                    .iter()
                    .find(|f| f.id == fleet)
                    .ok_or(WorldError::InvalidReference)?;
                if f.location != FleetLocation::InTransit(e.id)
                    || !system(origin)
                    || !system(destination)
                    || origin == destination
                {
                    return Err(WorldError::InvalidReference);
                }
                (2, fleet)
            }
        };
        if e.id != crate::schedules::schedule_id(g, e.operation, kind, subject) {
            return Err(WorldError::InvalidReference);
        }
    }
    Ok(())
}
fn sorted(ids: impl Iterator<Item = EntityId>) -> bool {
    let mut last = None;
    for id in ids {
        if last.is_some_and(|v| v >= id) {
            return false;
        }
        last = Some(id);
    }
    true
}
fn validate_resource_costs(s: &State) -> Result<(), WorldError> {
    for f in &s.factions {
        let colonies = s
            .markets
            .iter()
            .filter(|m| m.owner == Some(f.id) && m.ordinal % 2 == 1)
            .count() as u64;
        let industries = s
            .markets
            .iter()
            .filter(|m| m.owner == Some(f.id))
            .flat_map(|m| m.industries)
            .filter(|i| *i != IndustryState::Absent)
            .count() as u64;
        let credits = colonies * 100 + industries * 60;
        let supplies = colonies * 40 + industries * 20;
        let travel = s.fleets.iter().any(|v| {
            v.faction == f.id
                && v.location
                    != FleetLocation::Docked(crate::genesis::entity(
                        s.genesis,
                        1,
                        u32::from(f.ordinal),
                    ))
        });
        let residual = f
            .spent_supplies
            .checked_sub(supplies)
            .ok_or(WorldError::Resources)?;
        if f.spent_credits != credits || !residual.is_multiple_of(20) || (travel && residual < 20) {
            return Err(WorldError::Resources);
        }
    }
    Ok(())
}
