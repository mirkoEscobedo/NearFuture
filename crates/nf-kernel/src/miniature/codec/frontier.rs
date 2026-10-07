use super::*;
pub(super) fn encode_in(
    frontier: &MiniatureFrontier,
    budget: &mut Budget,
) -> MiniatureResult<Vec<u8>> {
    let rebuilt = admit_miniature(
        &frontier.snapshot,
        frontier.intents.clone(),
        frontier.authority,
        frontier.membership_revision,
    )?;
    if rebuilt != *frontier {
        return Err(MiniatureRejection::InvalidFrontier);
    }
    let mut w = Writer::new(3, budget)?;
    w.raw(&frontier.input_hash)?;
    w.u64(frontier.authority.term.0)?;
    w.u64(frontier.authority.session.0)?;
    w.u64(frontier.committing_tick.0)?;
    w.u64(frontier.membership_revision)?;
    let snapshot = snapshot::encode_in(&frontier.snapshot, w.budget)?;
    w.blob(&snapshot, MAX_MINIATURE_BYTES)?;
    w.count(frontier.intents.len())?;
    for intent in &frontier.intents {
        let bytes = intent::encode_in(intent, w.budget)?;
        w.blob(&bytes, MAX_MINIATURE_BYTES)?;
    }
    match frontier.reservation() {
        None => w.byte(0)?,
        Some(hold) => {
            w.byte(1)?;
            holds::write_hold(&mut w, hold)?;
        }
    }
    Ok(w.finish())
}
pub(super) fn decode_in(bytes: &[u8], budget: &mut Budget) -> MiniatureResult<MiniatureFrontier> {
    use nf_contract::identity::{AuthorityTerm, RuntimeSession, WorldTick};
    let mut r = Reader::new(bytes, 3, budget)?;
    let input_hash = r.fixed::<32>()?;
    let authority = crate::AuthorityContext {
        term: AuthorityTerm(r.u64()?),
        session: RuntimeSession(r.u64()?),
    };
    let committing_tick = WorldTick(r.u64()?);
    let membership_revision = r.u64()?;
    let snapshot_bytes = r.blob(MAX_MINIATURE_BYTES)?;
    let snapshot = snapshot::decode_in(snapshot_bytes, r.budget)?;
    let mut intents = Vec::new();
    for _ in 0..r.count(64)? {
        let value = r.blob(MAX_MINIATURE_BYTES)?;
        intents.push(intent::decode_in(value, r.budget)?);
    }
    let hold = match r.byte()? {
        0 => None,
        1 => Some(holds::read_hold(&mut r)?),
        _ => return Err(MiniatureRejection::InvalidValue),
    };
    r.done()?;
    let frontier = admit_miniature(&snapshot, intents, authority, membership_revision)?;
    if frontier.input_hash != input_hash
        || frontier.committing_tick != committing_tick
        || frontier.reservation() != hold
    {
        return Err(MiniatureRejection::InvalidFrontier);
    }
    if encode_in(&frontier, &mut Budget::default())? != bytes {
        return Err(MiniatureRejection::InvalidValue);
    }
    Ok(frontier)
}
