use crate::*;
use alloc::vec::Vec;
use nf_contract::identity::*;
use sha2::{Digest, Sha256};
pub(crate) fn schedule_id(
    g: Genesis,
    operation: OperationId,
    kind: u8,
    subject: EntityId,
) -> EntityId {
    let mut h = Sha256::new();
    h.update(b"NF-MINI-SCHEDULE-1\0");
    h.update(g.universe.as_bytes());
    h.update(g.history.as_bytes());
    h.update(g.seed);
    h.update(operation.as_bytes());
    h.update([kind]);
    h.update(subject.as_bytes());
    let d = h.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&d[..16]);
    EntityId::from_bytes(id)
}
pub fn due_events(state: &State, tick: WorldTick) -> Result<Vec<Schedule>, WorldError> {
    if state.schedules.len() > 32 {
        return Err(WorldError::Limit);
    }
    if state.schedules.iter().any(|s| s.due < tick) {
        return Err(WorldError::InvalidValue);
    }
    Ok(state
        .schedules
        .iter()
        .filter(|s| s.due == tick)
        .cloned()
        .collect())
}
pub fn apply_due(state: &State, tick: WorldTick, events: &[Schedule]) -> Result<State, WorldError> {
    if events.len() > 32 {
        return Err(WorldError::Limit);
    }
    if events != due_events(state, tick)? {
        return Err(WorldError::Malformed);
    }
    let mut next = state.clone();
    for event in events {
        match event.kind {
            ScheduleKind::Industry { market, industry } => {
                let m = next
                    .markets
                    .iter_mut()
                    .find(|m| m.id == market)
                    .ok_or(WorldError::InvalidReference)?;
                let slot = &mut m.industries[industry as usize - 1];
                if *slot != IndustryState::Constructing(event.id) {
                    return Err(WorldError::Locked);
                }
                *slot = IndustryState::Complete;
            }
            ScheduleKind::Arrival {
                fleet, destination, ..
            } => {
                let fleet = next
                    .fleets
                    .iter_mut()
                    .find(|v| v.id == fleet)
                    .ok_or(WorldError::InvalidReference)?;
                if fleet.location != FleetLocation::InTransit(event.id) {
                    return Err(WorldError::Locked);
                }
                fleet.location = FleetLocation::Docked(destination);
            }
        }
        next.schedules.retain(|s| s.id != event.id);
        next.revision = next.revision.checked_next().ok_or(WorldError::Overflow)?;
    }
    validate_at(&next, tick)?;
    Ok(next)
}
