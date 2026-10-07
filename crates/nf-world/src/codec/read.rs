use super::*;
pub fn decode_component(input: &[u8], tick: WorldTick) -> Result<State, WorldError> {
    let mut r = Reader::new(input)?;
    if &r.array::<11>()? != b"NF-CANON-1\0" {
        return Err(WorldError::Malformed);
    }
    if r.u16()? != 7 || r.u16()? != 5 || r.u16()? != 2 {
        return Err(WorldError::Unsupported);
    }
    let universe = UniverseId::from_bytes(r.array()?);
    let history = HistoryId::from_bytes(r.array()?);
    let seed = r.array()?;
    if r.array::<32>()? != ruleset_hash() {
        return Err(WorldError::Unsupported);
    }
    let accounts = [
        AccountId::from_bytes(r.array()?),
        AccountId::from_bytes(r.array()?),
        AccountId::from_bytes(r.array()?),
    ];
    let genesis = Genesis {
        universe,
        history,
        seed,
        accounts,
    };
    let revision = AggregateRevision(r.u64()?);
    let n = r.count(3, Some(3))?;
    let mut systems = Vec::with_capacity(n);
    for _ in 0..n {
        systems.push(System {
            id: r.id()?,
            ordinal: r.u8()?,
        });
    }
    let n = r.count(3, Some(3))?;
    let mut factions = Vec::with_capacity(n);
    for _ in 0..n {
        factions.push(Faction {
            id: r.id()?,
            ordinal: r.u8()?,
            account: AccountId::from_bytes(r.array()?),
            credits: r.u64()?,
            supplies: r.u64()?,
            spent_credits: r.u64()?,
            spent_supplies: r.u64()?,
        });
    }
    let n = r.count(6, Some(6))?;
    let mut markets = Vec::with_capacity(n);
    for _ in 0..n {
        let id = r.id()?;
        let ordinal = r.u8()?;
        let system = r.id()?;
        let owner = match r.u8()? {
            0 => None,
            1 => Some(r.id()?),
            _ => return Err(WorldError::Unsupported),
        };
        let mut industries = [IndustryState::Absent; 2];
        for slot in &mut industries {
            *slot = match r.u8()? {
                0 => IndustryState::Absent,
                1 => IndustryState::Constructing(r.id()?),
                2 => IndustryState::Complete,
                _ => return Err(WorldError::Unsupported),
            }
        }
        markets.push(Market {
            id,
            ordinal,
            system,
            owner,
            industries,
        });
    }
    let n = r.count(3, Some(3))?;
    let mut fleets = Vec::with_capacity(n);
    for _ in 0..n {
        let id = r.id()?;
        let faction = r.id()?;
        let location = match r.u8()? {
            1 => FleetLocation::Docked(r.id()?),
            2 => FleetLocation::InTransit(r.id()?),
            _ => return Err(WorldError::Unsupported),
        };
        fleets.push(Fleet {
            id,
            faction,
            location,
        });
    }
    let n = r.count(3, Some(3))?;
    let mut relations = Vec::with_capacity(n);
    for _ in 0..n {
        relations.push(Relation {
            id: r.id()?,
            left: r.id()?,
            right: r.id()?,
            score: i32::try_from(i64::from_le_bytes(r.array()?))
                .map_err(|_| WorldError::InvalidValue)?,
        });
    }
    let n = r.count(32, None)?;
    let mut schedules = Vec::with_capacity(n);
    for _ in 0..n {
        let id = r.id()?;
        let operation = OperationId::from_bytes(r.array()?);
        let due = WorldTick(r.u64()?);
        let kind = match r.u8()? {
            1 => ScheduleKind::Industry {
                market: r.id()?,
                industry: r.industry()?,
            },
            2 => ScheduleKind::Arrival {
                fleet: r.id()?,
                origin: r.id()?,
                destination: r.id()?,
            },
            _ => return Err(WorldError::Unsupported),
        };
        schedules.push(Schedule {
            id,
            operation,
            due,
            kind,
        });
    }
    if r.position != input.len() {
        return Err(WorldError::Malformed);
    }
    let s = State {
        genesis,
        revision,
        systems,
        factions,
        markets,
        fleets,
        relations,
        schedules,
    };
    validate_at(&s, tick)?;
    if encode_component(&s, tick)? != input {
        return Err(WorldError::Malformed);
    }
    Ok(s)
}
