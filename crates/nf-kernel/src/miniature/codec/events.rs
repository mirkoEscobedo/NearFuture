use super::*;
use nf_world::{Schedule, ScheduleKind};
pub(super) fn write_schedule(w: &mut Writer<'_>, event: &Schedule) -> MiniatureResult<()> {
    w.raw(event.id.as_bytes())?;
    w.raw(event.operation.as_bytes())?;
    w.u64(event.due.0)?;
    match event.kind {
        ScheduleKind::Industry { market, industry } => {
            w.byte(1)?;
            w.raw(market.as_bytes())?;
            w.byte(industry as u8)?;
        }
        ScheduleKind::Arrival {
            fleet,
            origin,
            destination,
        } => {
            w.byte(2)?;
            w.raw(fleet.as_bytes())?;
            w.raw(origin.as_bytes())?;
            w.raw(destination.as_bytes())?;
        }
    }
    Ok(())
}
pub(super) fn write_event(w: &mut Writer<'_>, event: &MiniatureActionEvent) -> MiniatureResult<()> {
    w.budget.charge(2)?;
    w.raw(event.request.as_bytes())?;
    w.raw(event.operation.as_bytes())?;
    w.raw(event.job.as_bytes())?;
    w.raw(event.actor.as_bytes())?;
    actions::write_action(w, &event.action)?;
    holds::write_hold(w, event.hold)?;
    match &event.created_schedule {
        None => w.byte(0)?,
        Some(schedule) => {
            w.budget.charge(1)?;
            w.byte(1)?;
            write_schedule(w, schedule)?;
        }
    }
    Ok(())
}
pub(super) fn read_schedule(r: &mut Reader<'_, '_>) -> MiniatureResult<Schedule> {
    use nf_contract::identity::{EntityId, OperationId, WorldTick};
    use nf_world::IndustryKind;
    let id = EntityId::from_bytes(r.fixed()?);
    let operation = OperationId::from_bytes(r.fixed()?);
    let due = WorldTick(r.u64()?);
    let kind = match r.byte()? {
        1 => ScheduleKind::Industry {
            market: EntityId::from_bytes(r.fixed()?),
            industry: match r.byte()? {
                1 => IndustryKind::Farming,
                2 => IndustryKind::Workshop,
                _ => return Err(MiniatureRejection::UnsupportedProvider),
            },
        },
        2 => ScheduleKind::Arrival {
            fleet: EntityId::from_bytes(r.fixed()?),
            origin: EntityId::from_bytes(r.fixed()?),
            destination: EntityId::from_bytes(r.fixed()?),
        },
        _ => return Err(MiniatureRejection::UnsupportedProvider),
    };
    Ok(Schedule {
        id,
        operation,
        due,
        kind,
    })
}
pub(super) fn read_event(r: &mut Reader<'_, '_>) -> MiniatureResult<MiniatureActionEvent> {
    use nf_contract::identity::*;
    r.budget.charge(2)?;
    let request = RequestId::from_bytes(r.fixed()?);
    let operation = OperationId::from_bytes(r.fixed()?);
    let job = JobId::from_bytes(r.fixed()?);
    let actor = AccountId::from_bytes(r.fixed()?);
    let action = actions::read_action(r)?;
    let hold = holds::read_hold(r)?;
    let created_schedule = match r.byte()? {
        0 => None,
        1 => {
            r.budget.charge(1)?;
            Some(read_schedule(r)?)
        }
        _ => return Err(MiniatureRejection::InvalidValue),
    };
    Ok(MiniatureActionEvent {
        request,
        operation,
        job,
        actor,
        action,
        hold,
        created_schedule,
    })
}
