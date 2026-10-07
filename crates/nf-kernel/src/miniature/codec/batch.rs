use super::*;
pub(super) fn encode_in(batch: &MiniatureBatch, budget: &mut Budget) -> MiniatureResult<Vec<u8>> {
    if batch.admitted.len() > 64
        || batch.outcomes.len() != batch.admitted.len()
        || batch.due.len() > 32
    {
        return Err(MiniatureRejection::Limit);
    }
    if batch.disposition == MiniatureDisposition::CancelPending
        && (batch.action_event.is_some() || !batch.due.is_empty())
    {
        return Err(MiniatureRejection::InvalidProposal);
    }
    let mut w = Writer::new(2, budget)?;
    w.raw(&batch.before_hash)?;
    w.raw(&batch.after_hash)?;
    w.byte(match batch.disposition {
        MiniatureDisposition::AdvanceTick => 1,
        MiniatureDisposition::CancelPending => 2,
    })?;
    w.u64(batch.tick.0)?;
    w.u64(batch.sequence.0)?;
    w.u64(batch.authority.term.0)?;
    w.u64(batch.authority.session.0)?;
    w.u64(batch.membership_revision)?;
    w.count(batch.admitted.len())?;
    for intent in &batch.admitted {
        let bytes = intent::encode_in(intent, w.budget)?;
        w.blob(&bytes, MAX_MINIATURE_BYTES)?;
    }
    match &batch.action_event {
        None => w.byte(0)?,
        Some(event) => {
            w.byte(1)?;
            events::write_event(&mut w, event)?;
        }
    }
    w.count(batch.due.len())?;
    for schedule in &batch.due {
        events::write_schedule(&mut w, schedule)?;
    }
    w.count(batch.outcomes.len())?;
    for outcome in &batch.outcomes {
        w.raw(outcome.operation.as_bytes())?;
        w.raw(outcome.job.as_bytes())?;
        w.byte(outcome.rejection.map_or(0, |r| r as u8))?;
    }
    Ok(w.finish())
}
pub(super) fn decode_in(
    bytes: &[u8],
    before: &MiniatureWorld,
    budget: &mut Budget,
) -> MiniatureResult<MiniatureBatch> {
    use nf_contract::identity::*;
    let mut r = Reader::new(bytes, 2, budget)?;
    let before_hash = r.fixed::<32>()?;
    if before_hash != miniature_state_hash(before)? {
        return Err(MiniatureRejection::StateMismatch);
    }
    let after_hash = r.fixed::<32>()?;
    let disposition = match r.byte()? {
        1 => MiniatureDisposition::AdvanceTick,
        2 => MiniatureDisposition::CancelPending,
        _ => return Err(MiniatureRejection::InvalidValue),
    };
    let tick = WorldTick(r.u64()?);
    let sequence = EventSeq(r.u64()?);
    let authority = crate::AuthorityContext {
        term: AuthorityTerm(r.u64()?),
        session: RuntimeSession(r.u64()?),
    };
    let membership_revision = r.u64()?;
    let mut admitted = Vec::new();
    for _ in 0..r.count(64)? {
        let value = r.blob(MAX_MINIATURE_BYTES)?;
        admitted.push(intent::decode_in(value, r.budget)?);
    }
    let action_event = match r.byte()? {
        0 => None,
        1 => Some(events::read_event(&mut r)?),
        _ => return Err(MiniatureRejection::InvalidValue),
    };
    let mut due = Vec::new();
    for _ in 0..r.count(32)? {
        due.push(events::read_schedule(&mut r)?);
    }
    let mut outcomes = Vec::new();
    for _ in 0..r.count(64)? {
        outcomes.push(MiniatureOutcome {
            operation: OperationId::from_bytes(r.fixed()?),
            job: JobId::from_bytes(r.fixed()?),
            rejection: rejection(r.byte()?)?,
        });
    }
    r.done()?;
    let batch = MiniatureBatch {
        before_hash,
        after_hash,
        disposition,
        tick,
        sequence,
        authority,
        membership_revision,
        admitted,
        action_event,
        due,
        outcomes,
    };
    replay_miniature(before, &batch)?;
    if encode_in(&batch, &mut Budget::default())? != bytes {
        return Err(MiniatureRejection::InvalidValue);
    }
    Ok(batch)
}
