use crate::*;
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use nf_contract::identity::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityContext {
    pub term: AuthorityTerm,
    pub session: RuntimeSession,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    SeekPeace { relation: EntityId },
    AdjustRelation { relation: EntityId, delta: i32 },
    AdjustMarket { market: EntityId, delta: i64 },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Intent {
    pub request: RequestId,
    pub operation: OperationId,
    pub job: JobId,
    pub actor: AccountId,
    pub universe: UniverseId,
    pub history: HistoryId,
    pub provider: ProviderId,
    pub expected: BTreeMap<AggregateId, AggregateRevision>,
    pub command: Command,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    pub(crate) intent: Intent,
    pub(crate) reads: BTreeMap<AggregateId, AggregateRevision>,
    pub(crate) writes: BTreeMap<AggregateId, AggregateRevision>,
    pub(crate) validation: Result<(), Rejection>,
}
impl Job {
    pub fn id(&self) -> JobId {
        self.intent.job
    }
    pub fn intent(&self) -> &Intent {
        &self.intent
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frontier {
    pub(crate) snapshot: World,
    pub(crate) input_hash: [u8; 32],
    pub(crate) authority: AuthorityContext,
    pub(crate) jobs: Vec<Job>,
}
impl Frontier {
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }
    pub fn input_hash(&self) -> [u8; 32] {
        self.input_hash
    }
    pub fn snapshot_world(&self) -> &World {
        &self.snapshot
    }
    pub fn authority(&self) -> AuthorityContext {
        self.authority
    }
    pub fn snapshot(&self) -> WorldView<'_> {
        self.snapshot.view()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    RelationChanged {
        relation: EntityId,
        delta: i32,
    },
    MarketAdjusted {
        market: EntityId,
        delta: i64,
    },
    ProviderAdvanced {
        provider: ProviderId,
        draws: u64,
        cooldown_until: WorldTick,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub job: JobId,
    pub provider: ProviderId,
    pub provider_version: u32,
    pub implementation_hash: [u8; 32],
    pub input_hash: [u8; 32],
    pub session: RuntimeSession,
    pub reads: BTreeMap<AggregateId, AggregateRevision>,
    pub writes: BTreeMap<AggregateId, AggregateRevision>,
    pub events: Vec<Event>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobResult {
    pub job: JobId,
    pub result: Result<Proposal, Rejection>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingPolicy {
    Pause,
    Recompute,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Settlement {
    Paused(Vec<JobId>),
    Committed {
        world: Box<World>,
        batch: CommittedBatch,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedBatch {
    pub before_hash: [u8; 32],
    pub after_hash: [u8; 32],
    pub tick: WorldTick,
    pub sequence: EventSeq,
    pub authority: AuthorityContext,
    pub admitted: Vec<Intent>,
    pub events: Vec<Event>,
    pub outcomes: Vec<Outcome>,
}

pub fn admit(
    world: &World,
    mut intents: Vec<Intent>,
    authority: AuthorityContext,
) -> Result<Frontier, Rejection> {
    if intents.len() > MAX_FRONTIER {
        return Err(Rejection::Limit);
    }
    let s = &world.spec;
    let snapshot_entries = s.factions.len()
        + s.relations.len()
        + s.markets.len()
        + s.providers.len()
        + s.registry.len()
        + s.ownership.len()
        + s.revisions.len()
        + world.outcomes.len()
        + s.registry
            .iter()
            .map(|m| {
                m.read_domains.len()
                    + m.write_domains.len()
                    + m.capabilities.len()
                    + m.principals.len()
            })
            .sum::<usize>();
    let frontier_entries = intents.len() + intents.iter().map(|v| v.expected.len()).sum::<usize>();
    if snapshot_entries
        .checked_add(frontier_entries)
        .ok_or(Rejection::Limit)?
        > 16384
    {
        return Err(Rejection::Limit);
    }
    let mut ids = BTreeSet::new();
    let mut operations = BTreeSet::new();
    let mut requests = BTreeSet::new();
    for intent in &intents {
        if intent.expected.len() > MAX_ENTITIES + MAX_PROVIDERS {
            return Err(Rejection::Limit);
        }
        if !ids.insert(intent.job)
            || !operations.insert(intent.operation)
            || !requests.insert(intent.request)
            || world
                .outcomes
                .iter()
                .any(|v| v.operation == intent.operation || v.job == intent.job)
        {
            return Err(Rejection::InvalidFrontier);
        }
    }
    intents.sort_by_key(|v| (v.provider, command_entity(&v.command), v.operation, v.job));
    let mut jobs = Vec::new();
    for intent in intents {
        let (reads, writes, validation) = match crate::provider::validate_intent(world, &intent) {
            Ok((r, w)) => (r, w, Ok(())),
            Err(e) => (BTreeMap::new(), BTreeMap::new(), Err(e)),
        };
        jobs.push(Job {
            intent,
            reads,
            writes,
            validation,
        });
    }
    Ok(Frontier {
        snapshot: world.clone(),
        input_hash: state_hash(world)?,
        authority,
        jobs,
    })
}
pub(crate) fn command_entity(command: &Command) -> EntityId {
    match command {
        Command::SeekPeace { relation } | Command::AdjustRelation { relation, .. } => *relation,
        Command::AdjustMarket { market, .. } => *market,
    }
}
pub fn settle(
    world: &World,
    frontier: &Frontier,
    results: Vec<JobResult>,
    authority: AuthorityContext,
    missing_policy: MissingPolicy,
) -> Result<Settlement, Rejection> {
    if results.len() > MAX_FRONTIER {
        return Err(Rejection::Limit);
    }
    if authority.term != frontier.authority.term {
        return Err(Rejection::FencedAuthority);
    }
    if authority.session != frontier.authority.session {
        return Err(Rejection::StaleSession);
    }
    if state_hash(world)? != frontier.input_hash {
        return Err(Rejection::StateMismatch);
    }
    let mut arrived = BTreeMap::new();
    for result in results {
        if !frontier.jobs.iter().any(|v| v.id() == result.job) {
            return Err(Rejection::UnknownJob);
        }
        if arrived.insert(result.job, result.result).is_some() {
            return Err(Rejection::DuplicateJob);
        }
    }
    let missing: Vec<_> = frontier
        .jobs
        .iter()
        .filter(|v| !arrived.contains_key(&v.id()))
        .map(Job::id)
        .collect();
    if !missing.is_empty() && missing_policy == MissingPolicy::Pause {
        return Ok(Settlement::Paused(missing));
    }
    for id in missing {
        arrived.insert(id, crate::evaluate(frontier, id));
    }
    let mut next = world.clone();
    let mut events = Vec::new();
    let mut outcomes = Vec::new();
    let mut accepted_reads = BTreeSet::new();
    let mut accepted_writes = BTreeSet::new();
    for job in &frontier.jobs {
        let result = arrived.get(&job.id()).ok_or(Rejection::InvalidFrontier)?;
        let conflict = job.reads.keys().any(|id| accepted_writes.contains(id))
            || job
                .writes
                .keys()
                .any(|id| accepted_reads.contains(id) || accepted_writes.contains(id));
        let accepted = (if conflict {
            Err(Rejection::Conflict)
        } else {
            crate::provider::events_for(frontier, job, result)
        })
        .and_then(|candidate| {
            crate::reducer::reduce_events(&next, &candidate).map(|state| (candidate, state))
        });
        let rejection = match accepted {
            Ok((candidate, state)) => {
                accepted_reads.extend(job.reads.keys().copied());
                accepted_writes.extend(job.writes.keys().copied());
                next = state;
                events.extend(candidate);
                None
            }
            Err(reason) => Some(reason),
        };
        outcomes.push(Outcome {
            operation: job.intent.operation,
            job: job.id(),
            rejection,
        });
    }
    next = crate::reducer::finish_tick(next, &outcomes)?;
    let batch = CommittedBatch {
        before_hash: frontier.input_hash,
        after_hash: state_hash(&next)?,
        tick: next.spec.tick,
        sequence: next.spec.event_seq,
        authority,
        admitted: frontier.jobs.iter().map(|v| v.intent.clone()).collect(),
        events,
        outcomes,
    };
    Ok(Settlement::Committed {
        world: Box::new(next),
        batch,
    })
}
