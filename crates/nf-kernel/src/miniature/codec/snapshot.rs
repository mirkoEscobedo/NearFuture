use super::*;
use nf_contract::identity::*;
pub(super) fn encode_in(world: &MiniatureWorld, budget: &mut Budget) -> MiniatureResult<Vec<u8>> {
    world.validate()?;
    let g = world.component.genesis();
    let m = world.metadata;
    let mut w = Writer::new(1, budget)?;
    w.raw(g.universe.as_bytes())?;
    w.raw(g.history.as_bytes())?;
    w.raw(&g.seed)?;
    w.raw(&nf_world::ruleset_hash())?;
    w.u64(m.tick.0)?;
    w.u64(m.event_sequence.0)?;
    w.raw(m.aggregate.as_bytes())?;
    w.raw(m.provider.as_bytes())?;
    w.raw(m.provider_aggregate.as_bytes())?;
    w.u64(m.provider_revision.0)?;
    w.budget.charge(65)?;
    w.blob(
        &nf_world::encode_component(&world.component, m.tick)?,
        MAX_MINIATURE_COMPONENT_BYTES,
    )?;
    w.count(world.outcomes.len())?;
    for outcome in &world.outcomes {
        w.raw(outcome.operation.as_bytes())?;
        w.raw(outcome.job.as_bytes())?;
        w.byte(outcome.rejection.map_or(0, |r| r as u8))?;
    }
    Ok(w.finish())
}
pub(super) fn decode_in(bytes: &[u8], budget: &mut Budget) -> MiniatureResult<MiniatureWorld> {
    let mut r = Reader::new(bytes, 1, budget)?;
    let universe = UniverseId::from_bytes(r.fixed()?);
    let history = HistoryId::from_bytes(r.fixed()?);
    let seed = r.fixed::<32>()?;
    if r.fixed::<32>()? != nf_world::ruleset_hash() {
        return Err(MiniatureRejection::UnsupportedProvider);
    }
    let metadata = MiniatureMetadata {
        tick: WorldTick(r.u64()?),
        event_sequence: EventSeq(r.u64()?),
        aggregate: AggregateId::from_bytes(r.fixed()?),
        provider: ProviderId::from_bytes(r.fixed()?),
        provider_aggregate: AggregateId::from_bytes(r.fixed()?),
        provider_revision: AggregateRevision(r.u64()?),
    };
    r.budget.charge(65)?;
    let component =
        nf_world::decode_component(r.blob(MAX_MINIATURE_COMPONENT_BYTES)?, metadata.tick)?;
    let g = component.genesis();
    if g.universe != universe || g.history != history || g.seed != seed {
        return Err(MiniatureRejection::InvalidReference);
    }
    let mut outcomes = Vec::new();
    for _ in 0..r.count(MAX_MINIATURE_OUTCOMES)? {
        outcomes.push(MiniatureOutcome {
            operation: OperationId::from_bytes(r.fixed()?),
            job: JobId::from_bytes(r.fixed()?),
            rejection: rejection(r.byte()?)?,
        });
    }
    r.done()?;
    let world = MiniatureWorld {
        metadata,
        component,
        outcomes,
    };
    world.validate()?;
    if encode_in(&world, &mut Budget::default())? != bytes {
        return Err(MiniatureRejection::InvalidValue);
    }
    Ok(world)
}
