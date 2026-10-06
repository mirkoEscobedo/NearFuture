use crate::*;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use nf_contract::identity::*;
use sha2::{Digest, Sha256};
mod primitives;
use primitives::*;
pub fn state_hash(world: &World) -> Result<[u8; 32], Rejection> {
    Ok(Sha256::digest(encode_snapshot(world)?).into())
}
pub fn encode_snapshot(world: &World) -> Result<Vec<u8>, Rejection> {
    encode_snapshot_in(world, &mut EntryBudget::default())
}
fn encode_snapshot_in(world: &World, budget: &mut EntryBudget) -> Result<Vec<u8>, Rejection> {
    let s = &world.spec;
    let mut w = Writer::header(1, budget);
    w.fixed(s.universe.as_bytes());
    w.fixed(s.history.as_bytes());
    w.fixed(&s.seed);
    w.fixed(&s.ruleset_hash);
    w.u64(s.tick.0);
    w.u64(s.event_seq.0);
    let mut items = s.factions.iter().collect::<Vec<_>>();
    items.sort_by_key(|v| v.id);
    w.count(items.len())?;
    for v in items {
        w.fixed(v.id.as_bytes());
        w.fixed(v.aggregate.as_bytes());
    }
    let mut items = s.relations.iter().collect::<Vec<_>>();
    items.sort_by_key(|v| v.id);
    w.count(items.len())?;
    for v in items {
        w.fixed(v.id.as_bytes());
        w.fixed(v.aggregate.as_bytes());
        w.fixed(v.left.as_bytes());
        w.fixed(v.right.as_bytes());
        w.i64(i64::from(v.score));
    }
    let mut items = s.markets.iter().collect::<Vec<_>>();
    items.sort_by_key(|v| v.id);
    w.count(items.len())?;
    for v in items {
        w.fixed(v.id.as_bytes());
        w.fixed(v.aggregate.as_bytes());
        w.fixed(v.faction.as_bytes());
        w.u64(v.credits);
    }
    let mut items = s.providers.iter().collect::<Vec<_>>();
    items.sort_by_key(|v| v.id);
    w.count(items.len())?;
    for v in items {
        w.fixed(v.id.as_bytes());
        w.fixed(v.aggregate.as_bytes());
        w.u64(v.draws);
        w.u64(v.cooldown_until.0);
    }
    let mut items = s.registry.iter().collect::<Vec<_>>();
    items.sort_by_key(|v| v.id);
    w.count(items.len())?;
    for v in items {
        w.fixed(v.id.as_bytes());
        w.fixed(&v.implementation_hash);
        w.u32(v.version);
        w.u32(v.state_schema);
        w.u32(v.kind as u32 + 1);
        w.count(v.read_domains.len())?;
        for d in &v.read_domains {
            w.u32(*d as u32 + 1);
        }
        w.count(v.write_domains.len())?;
        for d in &v.write_domains {
            w.u32(*d as u32 + 1);
        }
        w.count(v.capabilities.len())?;
        for c in &v.capabilities {
            w.u32(*c as u32 + 1);
        }
        w.count(v.principals.len())?;
        for id in &v.principals {
            w.fixed(id.as_bytes());
        }
        w.u32(v.max_events);
    }
    let mut items = s.ownership.iter().collect::<Vec<_>>();
    items.sort_by_key(|v| v.aggregate);
    w.count(items.len())?;
    for v in items {
        w.fixed(v.aggregate.as_bytes());
        w.fixed(v.provider.as_bytes());
        w.u64(v.generation);
        w.u64(v.activation_seq.0);
        w.fixed(&v.ruleset_hash);
    }
    w.count(s.revisions.len())?;
    for (id, revision) in &s.revisions {
        w.fixed(id.as_bytes());
        w.u64(revision.0);
    }
    w.count(world.outcomes.len())?;
    for v in &world.outcomes {
        w.fixed(v.operation.as_bytes());
        w.fixed(v.job.as_bytes());
        w.u32(v.rejection.map_or(0, |r| r as u32 + 1));
    }
    w.finish()
}
fn domain(n: u32) -> Result<Domain, Rejection> {
    match n {
        1 => Ok(Domain::Faction),
        2 => Ok(Domain::Relation),
        3 => Ok(Domain::Market),
        4 => Ok(Domain::Provider),
        _ => Err(Rejection::InvalidValue),
    }
}
fn capability(n: u32) -> Result<Capability, Rejection> {
    match n {
        1 => Ok(Capability::SeekPeace),
        2 => Ok(Capability::AdjustRelation),
        3 => Ok(Capability::AdjustMarket),
        _ => Err(Rejection::InvalidValue),
    }
}
fn rejection(n: u32) -> Result<Option<Rejection>, Rejection> {
    use Rejection::*;
    let codes = [
        InvalidReference,
        DuplicateIdentity,
        Limit,
        InvalidValue,
        UnsupportedProvider,
        Unauthorized,
        StaleRevision,
        Conflict,
        Overflow,
        InvalidProposal,
        ProviderFailed,
        StaleSession,
        FencedAuthority,
        UnknownJob,
        DuplicateJob,
        InvalidFrontier,
        StateMismatch,
    ];
    if n == 0 {
        Ok(None)
    } else {
        codes
            .get(n as usize - 1)
            .copied()
            .map(Some)
            .ok_or(InvalidValue)
    }
}
pub fn decode_snapshot(bytes: &[u8]) -> Result<World, Rejection> {
    decode_snapshot_in(bytes, &mut EntryBudget::default())
}
fn decode_snapshot_in(bytes: &[u8], budget: &mut EntryBudget) -> Result<World, Rejection> {
    let mut r = Reader::new(bytes, 1, budget)?;
    let mut s = WorldSpec::empty(
        UniverseId::from_bytes(r.fixed()?),
        HistoryId::from_bytes(r.fixed()?),
        r.fixed()?,
        r.fixed()?,
    );
    s.tick = WorldTick(r.u64()?);
    s.event_seq = EventSeq(r.u64()?);
    for _ in 0..r.count(MAX_ENTITIES)? {
        s.factions.push(Faction {
            id: EntityId::from_bytes(r.fixed()?),
            aggregate: AggregateId::from_bytes(r.fixed()?),
        });
    }
    for _ in 0..r.count(MAX_ENTITIES - s.factions.len())? {
        s.relations.push(Relation {
            id: EntityId::from_bytes(r.fixed()?),
            aggregate: AggregateId::from_bytes(r.fixed()?),
            left: EntityId::from_bytes(r.fixed()?),
            right: EntityId::from_bytes(r.fixed()?),
            score: r.i64()?.try_into().map_err(|_| Rejection::InvalidValue)?,
        });
    }
    for _ in 0..r.count(MAX_ENTITIES - s.factions.len() - s.relations.len())? {
        s.markets.push(Market {
            id: EntityId::from_bytes(r.fixed()?),
            aggregate: AggregateId::from_bytes(r.fixed()?),
            faction: EntityId::from_bytes(r.fixed()?),
            credits: r.u64()?,
        });
    }
    for _ in 0..r.count(MAX_PROVIDERS)? {
        s.providers.push(ProviderState {
            id: ProviderId::from_bytes(r.fixed()?),
            aggregate: AggregateId::from_bytes(r.fixed()?),
            draws: r.u64()?,
            cooldown_until: WorldTick(r.u64()?),
        });
    }
    for _ in 0..r.count(MAX_PROVIDERS)? {
        let id = ProviderId::from_bytes(r.fixed()?);
        let implementation_hash = r.fixed()?;
        let version = r.u32()?;
        let state_schema = r.u32()?;
        let kind = match r.u32()? {
            1 => ProviderKind::SyntheticPeace,
            2 => ProviderKind::Manual,
            _ => return Err(Rejection::UnsupportedProvider),
        };
        let mut read_domains = BTreeSet::new();
        for _ in 0..r.count(4)? {
            if !read_domains.insert(domain(r.u32()?)?) {
                return Err(Rejection::DuplicateIdentity);
            }
        }
        let mut write_domains = BTreeSet::new();
        for _ in 0..r.count(4)? {
            if !write_domains.insert(domain(r.u32()?)?) {
                return Err(Rejection::DuplicateIdentity);
            }
        }
        let mut capabilities = BTreeSet::new();
        for _ in 0..r.count(3)? {
            if !capabilities.insert(capability(r.u32()?)?) {
                return Err(Rejection::DuplicateIdentity);
            }
        }
        let mut principals = BTreeSet::new();
        for _ in 0..r.count(64)? {
            if !principals.insert(AccountId::from_bytes(r.fixed()?)) {
                return Err(Rejection::DuplicateIdentity);
            }
        }
        s.registry.push(ProviderManifest {
            id,
            implementation_hash,
            version,
            state_schema,
            kind,
            read_domains,
            write_domains,
            capabilities,
            principals,
            max_events: r.u32()?,
        });
    }
    for _ in 0..r.count(MAX_ENTITIES + MAX_PROVIDERS)? {
        s.ownership.push(OwnershipRecord {
            aggregate: AggregateId::from_bytes(r.fixed()?),
            provider: ProviderId::from_bytes(r.fixed()?),
            generation: r.u64()?,
            activation_seq: EventSeq(r.u64()?),
            ruleset_hash: r.fixed()?,
        });
    }
    s.revisions = BTreeMap::new();
    for _ in 0..r.count(MAX_ENTITIES + MAX_PROVIDERS)? {
        if s.revisions
            .insert(
                AggregateId::from_bytes(r.fixed()?),
                AggregateRevision(r.u64()?),
            )
            .is_some()
        {
            return Err(Rejection::DuplicateIdentity);
        }
    }
    let mut outcomes = Vec::new();
    for _ in 0..r.count(MAX_OUTCOMES)? {
        outcomes.push(Outcome {
            operation: OperationId::from_bytes(r.fixed()?),
            job: JobId::from_bytes(r.fixed()?),
            rejection: rejection(r.u32()?)?,
        });
    }
    let mut operations = BTreeSet::new();
    let mut jobs = BTreeSet::new();
    if outcomes
        .iter()
        .any(|v| !operations.insert(v.operation) || !jobs.insert(v.job))
    {
        return Err(Rejection::DuplicateIdentity);
    }
    r.done()?;
    let mut world = World::new(s)?;
    world.outcomes = outcomes;
    if encode_snapshot(&world)? != bytes {
        return Err(Rejection::InvalidValue);
    }
    Ok(world)
}
mod jobs;
pub use jobs::*;
