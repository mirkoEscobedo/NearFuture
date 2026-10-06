use super::{decode_snapshot_in, encode_snapshot_in, primitives::*, rejection};
use crate::*;
use alloc::{collections::BTreeMap, vec::Vec};
use nf_contract::identity::*;
use sha2::{Digest, Sha256};
pub fn intent_digest(intent: &Intent) -> Result<[u8; 32], Rejection> {
    Ok(Sha256::digest(encode_intent(intent)?).into())
}
pub fn encode_intent(intent: &Intent) -> Result<Vec<u8>, Rejection> {
    encode_intent_in(intent, &mut EntryBudget::default())
}
fn encode_intent_in(intent: &Intent, budget: &mut EntryBudget) -> Result<Vec<u8>, Rejection> {
    if intent.expected.len() > MAX_ENTITIES + MAX_PROVIDERS {
        return Err(Rejection::Limit);
    }
    let mut w = Writer::header(4, budget);
    for id in [
        intent.request.as_bytes(),
        intent.operation.as_bytes(),
        intent.job.as_bytes(),
        intent.actor.as_bytes(),
        intent.universe.as_bytes(),
        intent.history.as_bytes(),
        intent.provider.as_bytes(),
    ] {
        w.fixed(id);
    }
    w.count(intent.expected.len())?;
    for (id, revision) in &intent.expected {
        w.fixed(id.as_bytes());
        w.u64(revision.0);
    }
    match intent.command {
        Command::SeekPeace { relation } => {
            w.u32(1);
            w.fixed(relation.as_bytes());
        }
        Command::AdjustRelation { relation, delta } => {
            w.u32(2);
            w.fixed(relation.as_bytes());
            w.i64(i64::from(delta));
        }
        Command::AdjustMarket { market, delta } => {
            w.u32(3);
            w.fixed(market.as_bytes());
            w.i64(delta);
        }
    }
    w.finish()
}
pub fn decode_intent(bytes: &[u8]) -> Result<Intent, Rejection> {
    decode_intent_in(bytes, &mut EntryBudget::default())
}
fn decode_intent_in(bytes: &[u8], budget: &mut EntryBudget) -> Result<Intent, Rejection> {
    let mut r = Reader::new(bytes, 4, budget)?;
    let request = RequestId::from_bytes(r.fixed()?);
    let operation = OperationId::from_bytes(r.fixed()?);
    let job = JobId::from_bytes(r.fixed()?);
    let actor = AccountId::from_bytes(r.fixed()?);
    let universe = UniverseId::from_bytes(r.fixed()?);
    let history = HistoryId::from_bytes(r.fixed()?);
    let provider = ProviderId::from_bytes(r.fixed()?);
    let mut expected = BTreeMap::new();
    for _ in 0..r.count(MAX_ENTITIES + MAX_PROVIDERS)? {
        if expected
            .insert(
                AggregateId::from_bytes(r.fixed()?),
                AggregateRevision(r.u64()?),
            )
            .is_some()
        {
            return Err(Rejection::DuplicateIdentity);
        }
    }
    let kind = r.u32()?;
    let id = EntityId::from_bytes(r.fixed()?);
    let command = match kind {
        1 => Command::SeekPeace { relation: id },
        2 => Command::AdjustRelation {
            relation: id,
            delta: r.i64()?.try_into().map_err(|_| Rejection::InvalidValue)?,
        },
        3 => Command::AdjustMarket {
            market: id,
            delta: r.i64()?,
        },
        _ => return Err(Rejection::InvalidValue),
    };
    r.done()?;
    let intent = Intent {
        request,
        operation,
        job,
        actor,
        universe,
        history,
        provider,
        expected,
        command,
    };
    if encode_intent(&intent)? != bytes {
        return Err(Rejection::InvalidValue);
    }
    Ok(intent)
}
pub fn encode_frontier(frontier: &Frontier) -> Result<Vec<u8>, Rejection> {
    let mut budget = EntryBudget::default();
    let mut w = Writer::header(3, &mut budget);
    let snapshot = encode_snapshot_in(&frontier.snapshot, &mut *w.budget)?;
    w.bytes(&snapshot)?;
    w.fixed(&frontier.input_hash);
    w.u64(frontier.authority.term.0);
    w.u64(frontier.authority.session.0);
    w.count(frontier.jobs.len())?;
    for job in &frontier.jobs {
        let intent = encode_intent_in(&job.intent, &mut *w.budget)?;
        w.bytes(&intent)?;
    }
    w.finish()
}
pub fn decode_frontier(bytes: &[u8]) -> Result<Frontier, Rejection> {
    let mut budget = EntryBudget::default();
    let mut r = Reader::new(bytes, 3, &mut budget)?;
    let snapshot_bytes = r.bytes()?;
    let snapshot = decode_snapshot_in(snapshot_bytes, &mut *r.budget)?;
    let input_hash = r.fixed()?;
    let authority = AuthorityContext {
        term: AuthorityTerm(r.u64()?),
        session: RuntimeSession(r.u64()?),
    };
    let mut intents = Vec::new();
    for _ in 0..r.count(MAX_FRONTIER)? {
        let body = r.bytes()?;
        intents.push(decode_intent_in(body, &mut *r.budget)?);
    }
    r.done()?;
    let frontier = admit(&snapshot, intents, authority)?;
    if frontier.input_hash != input_hash || encode_frontier(&frontier)? != bytes {
        return Err(Rejection::InvalidFrontier);
    }
    Ok(frontier)
}
pub fn encode_batch(batch: &CommittedBatch) -> Result<Vec<u8>, Rejection> {
    if batch.admitted.len() > MAX_FRONTIER
        || batch.outcomes.len() > MAX_FRONTIER
        || batch.events.len() > MAX_FRONTIER * 2
    {
        return Err(Rejection::Limit);
    }
    let mut budget = EntryBudget::default();
    let mut w = Writer::header(2, &mut budget);
    w.fixed(&batch.before_hash);
    w.fixed(&batch.after_hash);
    w.u64(batch.tick.0);
    w.u64(batch.sequence.0);
    w.u64(batch.authority.term.0);
    w.u64(batch.authority.session.0);
    w.count(batch.admitted.len())?;
    for intent in &batch.admitted {
        let body = encode_intent_in(intent, &mut *w.budget)?;
        w.bytes(&body)?;
    }
    w.count(batch.events.len())?;
    for event in &batch.events {
        match event {
            Event::RelationChanged { relation, delta } => {
                w.u32(1);
                w.fixed(relation.as_bytes());
                w.i64(i64::from(*delta));
            }
            Event::MarketAdjusted { market, delta } => {
                w.u32(2);
                w.fixed(market.as_bytes());
                w.i64(*delta);
            }
            Event::ProviderAdvanced {
                provider,
                draws,
                cooldown_until,
            } => {
                w.u32(3);
                w.fixed(provider.as_bytes());
                w.u64(*draws);
                w.u64(cooldown_until.0);
            }
        }
    }
    w.count(batch.outcomes.len())?;
    for outcome in &batch.outcomes {
        w.fixed(outcome.operation.as_bytes());
        w.fixed(outcome.job.as_bytes());
        w.u32(outcome.rejection.map_or(0, |v| v as u32 + 1));
    }
    w.finish()
}
pub fn decode_batch(bytes: &[u8]) -> Result<CommittedBatch, Rejection> {
    let mut budget = EntryBudget::default();
    let mut r = Reader::new(bytes, 2, &mut budget)?;
    let before_hash = r.fixed()?;
    let after_hash = r.fixed()?;
    let tick = WorldTick(r.u64()?);
    let sequence = EventSeq(r.u64()?);
    let authority = AuthorityContext {
        term: AuthorityTerm(r.u64()?),
        session: RuntimeSession(r.u64()?),
    };
    let mut admitted = Vec::new();
    for _ in 0..r.count(MAX_FRONTIER)? {
        let body = r.bytes()?;
        admitted.push(decode_intent_in(body, &mut *r.budget)?);
    }
    let mut events = Vec::new();
    for _ in 0..r.count(MAX_FRONTIER * 2)? {
        events.push(match r.u32()? {
            1 => Event::RelationChanged {
                relation: EntityId::from_bytes(r.fixed()?),
                delta: r.i64()?.try_into().map_err(|_| Rejection::InvalidValue)?,
            },
            2 => Event::MarketAdjusted {
                market: EntityId::from_bytes(r.fixed()?),
                delta: r.i64()?,
            },
            3 => Event::ProviderAdvanced {
                provider: ProviderId::from_bytes(r.fixed()?),
                draws: r.u64()?,
                cooldown_until: WorldTick(r.u64()?),
            },
            _ => return Err(Rejection::InvalidValue),
        });
    }
    let mut outcomes = Vec::new();
    for _ in 0..r.count(MAX_FRONTIER)? {
        outcomes.push(Outcome {
            operation: OperationId::from_bytes(r.fixed()?),
            job: JobId::from_bytes(r.fixed()?),
            rejection: rejection(r.u32()?)?,
        });
    }
    r.done()?;
    let batch = CommittedBatch {
        before_hash,
        after_hash,
        tick,
        sequence,
        authority,
        admitted,
        events,
        outcomes,
    };
    if encode_batch(&batch)? != bytes {
        return Err(Rejection::InvalidValue);
    }
    Ok(batch)
}
