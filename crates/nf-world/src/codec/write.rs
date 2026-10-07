use super::*;
pub fn encode_component(s: &State, tick: WorldTick) -> Result<Vec<u8>, WorldError> {
    validate_at(s, tick)?;
    let mut b = Vec::with_capacity(4096);
    b.extend(b"NF-CANON-1\0");
    for value in [7u16, 5, 2] {
        b.extend(value.to_le_bytes());
    }
    let g = s.genesis;
    b.extend(g.universe.as_bytes());
    b.extend(g.history.as_bytes());
    b.extend(g.seed);
    b.extend(ruleset_hash());
    for account in g.accounts {
        b.extend(account.as_bytes());
    }
    b.extend(s.revision.0.to_le_bytes());
    count(&mut b, s.systems.len());
    for v in &s.systems {
        b.extend(v.id.as_bytes());
        b.push(v.ordinal);
    }
    count(&mut b, s.factions.len());
    for v in &s.factions {
        b.extend(v.id.as_bytes());
        b.push(v.ordinal);
        b.extend(v.account.as_bytes());
        for amount in [v.credits, v.supplies, v.spent_credits, v.spent_supplies] {
            b.extend(amount.to_le_bytes());
        }
    }
    count(&mut b, s.markets.len());
    for m in &s.markets {
        b.extend(m.id.as_bytes());
        b.push(m.ordinal);
        b.extend(m.system.as_bytes());
        match m.owner {
            None => b.push(0),
            Some(owner) => {
                b.push(1);
                b.extend(owner.as_bytes());
            }
        }
        for slot in m.industries {
            match slot {
                IndustryState::Absent => b.push(0),
                IndustryState::Constructing(id) => {
                    b.push(1);
                    b.extend(id.as_bytes());
                }
                IndustryState::Complete => b.push(2),
            }
        }
    }
    count(&mut b, s.fleets.len());
    for v in &s.fleets {
        b.extend(v.id.as_bytes());
        b.extend(v.faction.as_bytes());
        match v.location {
            FleetLocation::Docked(id) => {
                b.push(1);
                b.extend(id.as_bytes());
            }
            FleetLocation::InTransit(id) => {
                b.push(2);
                b.extend(id.as_bytes());
            }
        }
    }
    count(&mut b, s.relations.len());
    for v in &s.relations {
        b.extend(v.id.as_bytes());
        b.extend(v.left.as_bytes());
        b.extend(v.right.as_bytes());
        b.extend(i64::from(v.score).to_le_bytes());
    }
    count(&mut b, s.schedules.len());
    for v in &s.schedules {
        b.extend(v.id.as_bytes());
        b.extend(v.operation.as_bytes());
        b.extend(v.due.0.to_le_bytes());
        match v.kind {
            ScheduleKind::Industry { market, industry } => {
                b.push(1);
                b.extend(market.as_bytes());
                b.push(industry as u8);
            }
            ScheduleKind::Arrival {
                fleet,
                origin,
                destination,
            } => {
                b.push(2);
                for id in [fleet, origin, destination] {
                    b.extend(id.as_bytes());
                }
            }
        }
    }
    if b.len() > MAX_COMPONENT_BYTES {
        return Err(WorldError::Limit);
    }
    Ok(b)
}
