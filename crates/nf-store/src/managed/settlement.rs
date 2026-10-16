use super::{
    bytes::{Encoder, Reader, same},
    error::{ManagedError, Result},
};
use nf_contract::identity::*;
use nf_kernel::{Event, JobResult, MissingPolicy, Proposal, Rejection};
use std::collections::BTreeMap;

pub(super) fn rejection_code(value: Rejection) -> u32 {
    match value {
        Rejection::InvalidReference => 1,
        Rejection::DuplicateIdentity => 2,
        Rejection::Limit => 3,
        Rejection::InvalidValue => 4,
        Rejection::UnsupportedProvider => 5,
        Rejection::Unauthorized => 6,
        Rejection::StaleRevision => 7,
        Rejection::Conflict => 8,
        Rejection::Overflow => 9,
        Rejection::InvalidProposal => 10,
        Rejection::ProviderFailed => 11,
        Rejection::StaleSession => 12,
        Rejection::FencedAuthority => 13,
        Rejection::UnknownJob => 14,
        Rejection::DuplicateJob => 15,
        Rejection::InvalidFrontier => 16,
        Rejection::StateMismatch => 17,
    }
}
pub(super) fn rejection(code: u32) -> Result<Rejection> {
    match code {
        1 => Ok(Rejection::InvalidReference),
        2 => Ok(Rejection::DuplicateIdentity),
        3 => Ok(Rejection::Limit),
        4 => Ok(Rejection::InvalidValue),
        5 => Ok(Rejection::UnsupportedProvider),
        6 => Ok(Rejection::Unauthorized),
        7 => Ok(Rejection::StaleRevision),
        8 => Ok(Rejection::Conflict),
        9 => Ok(Rejection::Overflow),
        10 => Ok(Rejection::InvalidProposal),
        11 => Ok(Rejection::ProviderFailed),
        12 => Ok(Rejection::StaleSession),
        13 => Ok(Rejection::FencedAuthority),
        14 => Ok(Rejection::UnknownJob),
        15 => Ok(Rejection::DuplicateJob),
        16 => Ok(Rejection::InvalidFrontier),
        17 => Ok(Rejection::StateMismatch),
        _ => Err(ManagedError::Corrupt),
    }
}
fn write_revisions(
    w: &mut Encoder,
    values: &BTreeMap<AggregateId, AggregateRevision>,
) -> Result<()> {
    if values.len() > 160 {
        return Err(ManagedError::Limit);
    }
    w.u32(values.len().try_into().map_err(|_| ManagedError::Limit)?)?;
    for (id, revision) in values {
        w.raw(id.as_bytes())?;
        w.u64(revision.0)?;
    }
    Ok(())
}
fn read_revisions(r: &mut Reader<'_>) -> Result<BTreeMap<AggregateId, AggregateRevision>> {
    let count = r.u32()?;
    if count > 160 {
        return Err(ManagedError::Limit);
    }
    let mut out = BTreeMap::new();
    let mut previous = None;
    for _ in 0..count {
        let id = AggregateId::from_bytes(r.fixed()?);
        if previous.is_some_and(|old| old >= id) {
            return Err(ManagedError::Corrupt);
        }
        previous = Some(id);
        out.insert(id, AggregateRevision(r.u64()?));
    }
    Ok(out)
}
fn write_proposal(w: &mut Encoder, proposal: &Proposal) -> Result<()> {
    w.raw(proposal.job.as_bytes())?;
    w.raw(proposal.provider.as_bytes())?;
    w.u32(proposal.provider_version)?;
    w.raw(&proposal.implementation_hash)?;
    w.raw(&proposal.input_hash)?;
    w.u64(proposal.session.0)?;
    write_revisions(w, &proposal.reads)?;
    write_revisions(w, &proposal.writes)?;
    if proposal.events.len() > 64 {
        return Err(ManagedError::Limit);
    }
    w.u32(
        proposal
            .events
            .len()
            .try_into()
            .map_err(|_| ManagedError::Limit)?,
    )?;
    for event in &proposal.events {
        match event {
            Event::RelationChanged { relation, delta } => {
                w.byte(1)?;
                w.raw(relation.as_bytes())?;
                w.raw(&delta.to_le_bytes())?;
            }
            Event::MarketAdjusted { market, delta } => {
                w.byte(2)?;
                w.raw(market.as_bytes())?;
                w.raw(&delta.to_le_bytes())?;
            }
            Event::ProviderAdvanced {
                provider,
                draws,
                cooldown_until,
            } => {
                w.byte(3)?;
                w.raw(provider.as_bytes())?;
                w.u64(*draws)?;
                w.u64(cooldown_until.0)?;
            }
        }
    }
    Ok(())
}
fn read_proposal(r: &mut Reader<'_>) -> Result<Proposal> {
    let mut value = Proposal {
        job: JobId::from_bytes(r.fixed()?),
        provider: ProviderId::from_bytes(r.fixed()?),
        provider_version: r.u32()?,
        implementation_hash: r.fixed()?,
        input_hash: r.fixed()?,
        session: RuntimeSession(r.u64()?),
        reads: read_revisions(r)?,
        writes: read_revisions(r)?,
        events: Vec::new(),
    };
    let count = r.u32()?;
    if count > 64 {
        return Err(ManagedError::Limit);
    }
    for _ in 0..count {
        value.events.push(match r.byte()? {
            1 => Event::RelationChanged {
                relation: EntityId::from_bytes(r.fixed()?),
                delta: i32::from_le_bytes(r.fixed()?),
            },
            2 => Event::MarketAdjusted {
                market: EntityId::from_bytes(r.fixed()?),
                delta: i64::from_le_bytes(r.fixed()?),
            },
            3 => Event::ProviderAdvanced {
                provider: ProviderId::from_bytes(r.fixed()?),
                draws: r.u64()?,
                cooldown_until: WorldTick(r.u64()?),
            },
            _ => return Err(ManagedError::Corrupt),
        });
    }
    Ok(value)
}
pub(super) fn encode(results: &[JobResult], missing: MissingPolicy) -> Result<Vec<u8>> {
    if results.len() > 64 {
        return Err(ManagedError::Limit);
    }
    let mut ordered = results.iter().collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|r| r.job);
    if ordered.windows(2).any(|p| p[0].job == p[1].job) {
        return Err(ManagedError::OperationConflict);
    }
    let mut w = Encoder::new(b"NF-ACTIVE-SETTLEMENT-1\0", 1_048_576)?;
    w.byte(match missing {
        MissingPolicy::Pause => 1,
        MissingPolicy::Recompute => 2,
    })?;
    w.u32(ordered.len().try_into().map_err(|_| ManagedError::Limit)?)?;
    for result in ordered {
        w.raw(result.job.as_bytes())?;
        match &result.result {
            Err(error) => {
                w.byte(1)?;
                w.u32(rejection_code(*error))?;
            }
            Ok(proposal) => {
                if proposal.job != result.job {
                    return Err(ManagedError::Corrupt);
                }
                w.byte(2)?;
                write_proposal(&mut w, proposal)?;
            }
        }
    }
    Ok(w.finish())
}
pub(super) fn decode(bytes: &[u8]) -> Result<(Vec<JobResult>, MissingPolicy)> {
    let mut r = Reader::new(bytes, b"NF-ACTIVE-SETTLEMENT-1\0", 1_048_576)?;
    let missing = match r.byte()? {
        1 => MissingPolicy::Pause,
        2 => MissingPolicy::Recompute,
        _ => return Err(ManagedError::Corrupt),
    };
    let count = r.u32()?;
    if count > 64 {
        return Err(ManagedError::Limit);
    }
    let mut results = Vec::new();
    let mut previous = None;
    for _ in 0..count {
        let job = JobId::from_bytes(r.fixed()?);
        if previous.is_some_and(|old| old >= job) {
            return Err(ManagedError::Corrupt);
        }
        previous = Some(job);
        let result = match r.byte()? {
            1 => Err(rejection(r.u32()?)?),
            2 => Ok(read_proposal(&mut r)?),
            _ => return Err(ManagedError::Corrupt),
        };
        if result.as_ref().is_ok_and(|p| p.job != job) {
            return Err(ManagedError::Corrupt);
        }
        results.push(JobResult { job, result });
    }
    r.finish()?;
    same(bytes, &encode(&results, missing)?)?;
    Ok((results, missing))
}
