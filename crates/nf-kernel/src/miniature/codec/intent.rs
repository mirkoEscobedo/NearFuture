use super::*;
pub(super) fn encode_in(intent: &MiniatureIntent, budget: &mut Budget) -> MiniatureResult<Vec<u8>> {
    validate_shape(intent)?;
    let mut w = Writer::new(4, budget)?;
    w.raw(intent.request.as_bytes())?;
    w.raw(intent.operation.as_bytes())?;
    w.raw(intent.job.as_bytes())?;
    w.raw(intent.actor.as_bytes())?;
    w.raw(intent.device.as_bytes())?;
    w.raw(intent.universe.as_bytes())?;
    w.raw(intent.history.as_bytes())?;
    w.raw(intent.provider.as_bytes())?;
    w.count(intent.expected.len())?;
    for (id, rev) in &intent.expected {
        w.raw(id.as_bytes())?;
        w.u64(rev.0)?;
    }
    actions::write_action(&mut w, &intent.action)?;
    Ok(w.finish())
}
pub(crate) fn validate_shape(intent: &MiniatureIntent) -> MiniatureResult<()> {
    if intent.expected.len() != 2 {
        return Err(MiniatureRejection::InvalidReference);
    }
    if matches!(intent.action,nf_world::WorldAction::SetRelationship{score,..}if!(-10000..=10000).contains(&score))
    {
        return Err(MiniatureRejection::InvalidValue);
    }
    Ok(())
}
pub(super) fn decode_in(bytes: &[u8], budget: &mut Budget) -> MiniatureResult<MiniatureIntent> {
    use alloc::collections::BTreeMap;
    use nf_contract::identity::*;
    let mut r = Reader::new(bytes, 4, budget)?;
    let request = RequestId::from_bytes(r.fixed()?);
    let operation = OperationId::from_bytes(r.fixed()?);
    let job = JobId::from_bytes(r.fixed()?);
    let actor = AccountId::from_bytes(r.fixed()?);
    let device = DeviceId::from_bytes(r.fixed()?);
    let universe = UniverseId::from_bytes(r.fixed()?);
    let history = HistoryId::from_bytes(r.fixed()?);
    let provider = ProviderId::from_bytes(r.fixed()?);
    if r.count(2)? != 2 {
        return Err(MiniatureRejection::InvalidReference);
    }
    let mut expected = BTreeMap::new();
    let mut previous = None;
    for _ in 0..2 {
        let id = AggregateId::from_bytes(r.fixed()?);
        let revision = AggregateRevision(r.u64()?);
        if previous.is_some_and(|last| last >= id) {
            return Err(MiniatureRejection::InvalidValue);
        }
        previous = Some(id);
        expected.insert(id, revision);
    }
    let intent = MiniatureIntent {
        request,
        operation,
        job,
        actor,
        device,
        universe,
        history,
        provider,
        expected,
        action: actions::read_action(&mut r)?,
    };
    r.done()?;
    validate_shape(&intent)?;
    if encode_in(&intent, &mut Budget::default())? != bytes {
        return Err(MiniatureRejection::InvalidValue);
    }
    Ok(intent)
}
