use super::*;
pub fn settle_miniature(
    world: &MiniatureWorld,
    frontier: &MiniatureFrontier,
    authority: crate::AuthorityContext,
) -> MiniatureResult<MiniatureBatch> {
    advance(world, frontier, authority).map(|(_, batch)| batch)
}
pub(super) fn advance(
    world: &MiniatureWorld,
    frontier: &MiniatureFrontier,
    authority: crate::AuthorityContext,
) -> MiniatureResult<(MiniatureWorld, MiniatureBatch)> {
    if world != &frontier.snapshot || miniature_state_hash(world)? != frontier.input_hash {
        return Err(MiniatureRejection::StateMismatch);
    }
    if authority.term != frontier.authority.term {
        return Err(MiniatureRejection::FencedAuthority);
    }
    if authority.session != frontier.authority.session {
        return Err(MiniatureRejection::StaleSession);
    }
    let sequence = world
        .metadata
        .event_sequence
        .checked_next()
        .ok_or(MiniatureRejection::Overflow)?;
    let mut component = world.component.clone();
    let mut action_event = None;
    if let Some(plan) = &frontier.plan {
        component = nf_world::apply_plan(&component, frontier.committing_tick, plan)?;
        let c = plan.candidate();
        let h = plan.reservation();
        action_event = Some(MiniatureActionEvent {
            request: c.request,
            operation: c.operation,
            job: c.job,
            actor: c.actor,
            action: c.action.clone(),
            hold: MiniatureHold {
                operation: h.operation(),
                faction: h.faction(),
                credits: h.credits(),
                supplies: h.supplies(),
            },
            created_schedule: component
                .schedules()
                .iter()
                .find(|s| s.operation == c.operation)
                .cloned(),
        });
    }
    let due = nf_world::due_events(&component, frontier.committing_tick)?;
    component = nf_world::apply_due(&component, frontier.committing_tick, &due)?;
    let metadata = MiniatureMetadata {
        tick: frontier.committing_tick,
        event_sequence: sequence,
        provider_revision: world
            .metadata
            .provider_revision
            .checked_next()
            .ok_or(MiniatureRejection::Overflow)?,
        ..world.metadata
    };
    let next = finish(component, metadata, &world.outcomes, &frontier.outcomes)?;
    let batch = MiniatureBatch {
        before_hash: frontier.input_hash,
        after_hash: miniature_state_hash(&next)?,
        disposition: MiniatureDisposition::AdvanceTick,
        tick: metadata.tick,
        sequence,
        authority,
        membership_revision: frontier.membership_revision,
        admitted: frontier.intents.clone(),
        action_event,
        due,
        outcomes: frontier.outcomes.clone(),
    };
    Ok((next, batch))
}
pub(super) fn finish(
    component: nf_world::State,
    metadata: MiniatureMetadata,
    previous: &[MiniatureOutcome],
    delta: &[MiniatureOutcome],
) -> MiniatureResult<MiniatureWorld> {
    if previous
        .len()
        .checked_add(delta.len())
        .is_none_or(|n| n > MAX_MINIATURE_OUTCOMES)
    {
        return Err(MiniatureRejection::Limit);
    }
    let mut outcomes = previous.to_vec();
    outcomes.extend_from_slice(delta);
    outcomes.sort_by_key(|o| (o.operation, o.job));
    let world = MiniatureWorld {
        metadata,
        component,
        outcomes,
    };
    world.validate()?;
    Ok(world)
}
/// Verify exact recorded transitions using pure NF rules; authentication and durable ordering are outside this core.
pub fn replay_miniature(
    world: &MiniatureWorld,
    batch: &MiniatureBatch,
) -> MiniatureResult<MiniatureWorld> {
    if miniature_state_hash(world)? != batch.before_hash {
        return Err(MiniatureRejection::StateMismatch);
    }
    let frontier = admit_miniature(
        world,
        batch.admitted.clone(),
        batch.authority,
        batch.membership_revision,
    )?;
    let (next, expected) = match batch.disposition {
        MiniatureDisposition::AdvanceTick => advance(world, &frontier, batch.authority)?,
        MiniatureDisposition::CancelPending => {
            let reason = batch
                .outcomes
                .first()
                .map_or(Some(MiniatureRejection::Cancelled), |o| o.rejection)
                .ok_or(MiniatureRejection::InvalidProposal)?;
            super::cancel::cancel(
                world,
                &frontier,
                batch.authority,
                batch.membership_revision,
                reason,
            )?
        }
    };
    if expected != *batch {
        return Err(MiniatureRejection::InvalidProposal);
    }
    Ok(next)
}
