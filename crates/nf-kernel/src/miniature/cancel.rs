use super::*;
pub fn cancel_miniature(
    world: &MiniatureWorld,
    frontier: &MiniatureFrontier,
    authority: crate::AuthorityContext,
    membership_revision: u64,
    reason: MiniatureRejection,
) -> MiniatureResult<MiniatureBatch> {
    cancel(world, frontier, authority, membership_revision, reason).map(|(_, batch)| batch)
}
pub(super) fn cancel(
    world: &MiniatureWorld,
    frontier: &MiniatureFrontier,
    authority: crate::AuthorityContext,
    membership_revision: u64,
    reason: MiniatureRejection,
) -> MiniatureResult<(MiniatureWorld, MiniatureBatch)> {
    if world != &frontier.snapshot || miniature_state_hash(world)? != frontier.input_hash {
        return Err(MiniatureRejection::StateMismatch);
    }
    if !matches!(
        reason,
        MiniatureRejection::AdmissionChanged | MiniatureRejection::Cancelled
    ) {
        return Err(MiniatureRejection::InvalidValue);
    }
    if authority.term.0 == 0 || authority.term < frontier.authority.term {
        return Err(MiniatureRejection::FencedAuthority);
    }
    if authority.session.0 == 0
        || (authority.term == frontier.authority.term
            && authority.session != frontier.authority.session)
    {
        return Err(MiniatureRejection::StaleSession);
    }
    if membership_revision < frontier.membership_revision {
        return Err(MiniatureRejection::StaleRevision);
    }
    let sequence = world
        .metadata
        .event_sequence
        .checked_next()
        .ok_or(MiniatureRejection::Overflow)?;
    let outcomes: alloc::vec::Vec<_> = frontier
        .intents
        .iter()
        .map(|i| MiniatureOutcome {
            operation: i.operation,
            job: i.job,
            rejection: Some(reason),
        })
        .collect();
    let metadata = MiniatureMetadata {
        event_sequence: sequence,
        ..world.metadata
    };
    let next = super::reducer::finish(
        world.component.clone(),
        metadata,
        &world.outcomes,
        &outcomes,
    )?;
    let batch = MiniatureBatch {
        before_hash: frontier.input_hash,
        after_hash: miniature_state_hash(&next)?,
        disposition: MiniatureDisposition::CancelPending,
        tick: metadata.tick,
        sequence,
        authority,
        membership_revision,
        admitted: frontier.intents.clone(),
        action_event: None,
        due: alloc::vec::Vec::new(),
        outcomes,
    };
    Ok((next, batch))
}
