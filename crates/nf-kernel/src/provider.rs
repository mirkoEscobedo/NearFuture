use crate::*;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec,
    vec::Vec,
};
use nf_contract::identity::*;
pub(crate) type Access = (
    BTreeMap<AggregateId, AggregateRevision>,
    BTreeMap<AggregateId, AggregateRevision>,
);
pub(crate) fn validate_intent(world: &World, intent: &Intent) -> Result<Access, Rejection> {
    let s = &world.spec;
    let manifest = s
        .registry
        .iter()
        .find(|v| v.id == intent.provider)
        .ok_or(Rejection::Unauthorized)?;
    if intent.universe != s.universe
        || intent.history != s.history
        || !manifest.principals.contains(&intent.actor)
    {
        return Err(Rejection::Unauthorized);
    }
    let module = world
        .view()
        .provider(intent.provider)
        .ok_or(Rejection::InvalidReference)?;
    let (target, domain, capability, factions) = match intent.command {
        Command::SeekPeace { relation } | Command::AdjustRelation { relation, .. } => {
            let v = world
                .view()
                .relation(relation)
                .ok_or(Rejection::InvalidReference)?;
            (
                v.aggregate,
                Domain::Relation,
                if matches!(intent.command, Command::SeekPeace { .. }) {
                    Capability::SeekPeace
                } else {
                    Capability::AdjustRelation
                },
                vec![v.left, v.right],
            )
        }
        Command::AdjustMarket { market, .. } => {
            let v = world
                .view()
                .market(market)
                .ok_or(Rejection::InvalidReference)?;
            (
                v.aggregate,
                Domain::Market,
                Capability::AdjustMarket,
                vec![v.faction],
            )
        }
    };
    if !manifest.capabilities.contains(&capability)
        || ![Domain::Faction, domain, Domain::Provider]
            .iter()
            .all(|d| manifest.read_domains.contains(d))
        || ![domain, Domain::Provider]
            .iter()
            .all(|d| manifest.write_domains.contains(d))
    {
        return Err(Rejection::Unauthorized);
    }
    let mut read_ids = BTreeSet::from([target, module.aggregate]);
    for faction in factions {
        read_ids.insert(
            s.factions
                .iter()
                .find(|v| v.id == faction)
                .ok_or(Rejection::InvalidReference)?
                .aggregate,
        );
    }
    let mut reads = BTreeMap::new();
    for id in read_ids {
        reads.insert(
            id,
            world
                .view()
                .revision(id)
                .ok_or(Rejection::InvalidReference)?,
        );
    }
    let writes = BTreeMap::from([
        (target, reads[&target]),
        (module.aggregate, reads[&module.aggregate]),
    ]);
    if writes.keys().any(|id| {
        !s.ownership
            .iter()
            .any(|o| o.aggregate == *id && o.provider == intent.provider)
    }) {
        return Err(Rejection::Unauthorized);
    }
    if intent.expected.keys().ne(reads.keys()) {
        return Err(Rejection::InvalidProposal);
    }
    if intent.expected != reads {
        return Err(Rejection::StaleRevision);
    }
    Ok((reads, writes))
}
pub fn evaluate(frontier: &Frontier, id: JobId) -> Result<Proposal, Rejection> {
    let job = frontier
        .jobs
        .iter()
        .find(|v| v.id() == id)
        .ok_or(Rejection::UnknownJob)?;
    job.validation?;
    let s = &frontier.snapshot.spec;
    let module = frontier
        .snapshot
        .view()
        .provider(job.intent.provider)
        .ok_or(Rejection::InvalidReference)?;
    let manifest = s
        .registry
        .iter()
        .find(|v| v.id == module.id)
        .ok_or(Rejection::Unauthorized)?;
    if module.cooldown_until > s.tick {
        return Err(Rejection::InvalidValue);
    }
    let mut draws = module.draws;
    let event = match job.intent.command {
        Command::SeekPeace { relation } => {
            let random = scoped_draw(&RngScope {
                seed: s.seed,
                history: s.history,
                ruleset_hash: s.ruleset_hash,
                provider: module.id,
                entity: relation,
                tick: s.tick,
                operation: job.intent.operation,
                draw: draws,
            });
            draws = draws.checked_add(1).ok_or(Rejection::Overflow)?;
            Event::RelationChanged {
                relation,
                delta: (random % 10 + 1) as i32,
            }
        }
        Command::AdjustRelation { relation, delta } => Event::RelationChanged { relation, delta },
        Command::AdjustMarket { market, delta } => Event::MarketAdjusted { market, delta },
    };
    let events = vec![
        event,
        Event::ProviderAdvanced {
            provider: module.id,
            draws,
            cooldown_until: s.tick.checked_next().ok_or(Rejection::Overflow)?,
        },
    ];
    if events.len() > manifest.max_events as usize {
        return Err(Rejection::Limit);
    }
    Ok(Proposal {
        job: id,
        provider: module.id,
        provider_version: manifest.version,
        implementation_hash: manifest.implementation_hash,
        input_hash: frontier.input_hash,
        session: frontier.authority.session,
        reads: job.reads.clone(),
        writes: job.writes.clone(),
        events,
    })
}
pub(crate) fn events_for(
    frontier: &Frontier,
    job: &Job,
    result: &Result<Proposal, Rejection>,
) -> Result<Vec<Event>, Rejection> {
    job.validation?;
    let expected = evaluate(frontier, job.id())?;
    let proposal = result.as_ref().map_err(|_| Rejection::ProviderFailed)?;
    if proposal.session != frontier.authority.session {
        return Err(Rejection::StaleSession);
    }
    if proposal
        .writes
        .keys()
        .any(|id| !job.writes.contains_key(id))
    {
        return Err(Rejection::Unauthorized);
    }
    if proposal.reads.keys().ne(job.reads.keys()) || proposal.writes.keys().ne(job.writes.keys()) {
        return Err(Rejection::InvalidProposal);
    }
    if proposal.reads != job.reads || proposal.writes != job.writes {
        return Err(Rejection::StaleRevision);
    }
    if proposal != &expected {
        return Err(Rejection::InvalidProposal);
    }
    Ok(proposal.events.clone())
}
