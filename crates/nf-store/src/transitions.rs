use crate::{
    error::{Result, StoreError},
    model::*,
};
use nf_contract::identity::*;
use nf_kernel::{Command, CommittedBatch, Frontier, Intent};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) fn prepare(
    old: &State,
    frontier: &Frontier,
    devices: &[PrincipalDevice],
    reservations: &[Reservation],
) -> Result<State> {
    if old.pending.is_some() {
        return Err(StoreError::InvalidTransition);
    }
    if frontier.snapshot_world() != &old.world
        || frontier.input_hash() != nf_kernel::state_hash(&old.world)?
    {
        return Err(StoreError::InvalidTransition);
    }
    if devices.len() != frontier.jobs().len() || reservations.len() > MAX_RESERVATIONS {
        return Err(StoreError::Limit);
    }
    let mut device_map = BTreeMap::new();
    for entry in devices {
        if device_map.insert(entry.request, entry.device).is_some() {
            return Err(StoreError::RequestConflict);
        }
    }
    if old
        .requests
        .len()
        .checked_add(devices.len())
        .is_none_or(|count| count > MAX_REQUESTS)
    {
        return Err(StoreError::Limit);
    }
    let mut next = old.clone();
    for job in frontier.jobs() {
        let intent = job.intent();
        let device = *device_map
            .get(&intent.request)
            .ok_or(StoreError::InvalidTransition)?;
        if old.requests.contains_key(&intent.request) {
            return Err(StoreError::RequestConflict);
        }
        next.requests.insert(
            intent.request,
            RequestRecord {
                intent: intent.clone(),
                device,
                status: RequestStatus::Pending {
                    operation: intent.operation,
                },
            },
        );
    }
    next.pending = Some(frontier.clone());
    for reservation in reservations {
        if next
            .reservations
            .insert(reservation.operation, *reservation)
            .is_some()
        {
            return Err(StoreError::InvalidTransition);
        }
    }
    next.validate()?;
    Ok(next)
}
pub(crate) fn validate_reservations(state: &State) -> Result<()> {
    let mut totals: BTreeMap<EntityId, u64> = BTreeMap::new();
    for (operation, reservation) in &state.reservations {
        if *operation != reservation.operation || reservation.amount == 0 {
            return Err(StoreError::InvalidTransition);
        }
        let request = state
            .requests
            .values()
            .find(|r| matches!(r.status,RequestStatus::Pending{operation:op}if op==*operation))
            .ok_or(StoreError::InvalidTransition)?;
        let Command::AdjustMarket { market, delta } = request.intent.command else {
            return Err(StoreError::InvalidTransition);
        };
        if market != reservation.market || delta >= 0 || delta.unsigned_abs() != reservation.amount
        {
            return Err(StoreError::InvalidTransition);
        }
        let total = totals.entry(market).or_default();
        *total = total
            .checked_add(reservation.amount)
            .ok_or(StoreError::Limit)?;
        if *total
            > state
                .world
                .view()
                .market(market)
                .ok_or(StoreError::InvalidTransition)?
                .credits
        {
            return Err(StoreError::InvalidTransition);
        }
    }
    Ok(())
}
pub(crate) fn commit(old: &State, batch: &CommittedBatch) -> Result<State> {
    let frontier = old.pending.as_ref().ok_or(StoreError::InvalidTransition)?;
    if batch
        .admitted
        .iter()
        .ne(frontier.jobs().iter().map(|job| job.intent()))
        || batch.authority != frontier.authority()
    {
        return Err(StoreError::InvalidTransition);
    }
    let mut next = old.clone();
    next.world = nf_kernel::apply_batch(&old.world, batch)?;
    next.pending = None;
    next.reservations.clear();
    let digest: [u8; 32] = Sha256::digest(nf_kernel::encode_batch(batch)?).into();
    if old
        .outbox
        .len()
        .checked_add(batch.outcomes.len())
        .is_none_or(|count| count > MAX_OUTBOX)
    {
        return Err(StoreError::Backpressure);
    }
    let mut operations = BTreeSet::new();
    for (intent, outcome) in batch.admitted.iter().zip(&batch.outcomes) {
        if outcome.operation != intent.operation
            || outcome.job != intent.job
            || !operations.insert(outcome.operation)
        {
            return Err(StoreError::InvalidTransition);
        }
        let request = next
            .requests
            .get_mut(&intent.request)
            .ok_or(StoreError::InvalidTransition)?;
        if request.intent != *intent || !matches!(request.status, RequestStatus::Pending { .. }) {
            return Err(StoreError::RequestConflict);
        }
        request.status = RequestStatus::Committed {
            operation: intent.operation,
            sequence: batch.sequence,
            rejection: outcome.rejection,
        };
        if next
            .outbox
            .insert(
                intent.operation,
                OutboxRecord {
                    operation: intent.operation,
                    sequence: batch.sequence,
                    batch_digest: digest,
                    rejection: outcome.rejection,
                },
            )
            .is_some()
        {
            return Err(StoreError::InvalidTransition);
        }
    }
    next.validate()?;
    Ok(next)
}
pub(crate) fn query(
    state: &State,
    intent: &Intent,
    device: DeviceId,
) -> Result<Option<RequestStatus>> {
    let Some(record) = state.requests.get(&intent.request) else {
        return Ok(None);
    };
    binding(&record.intent, record.device)?
        .verify_retry(
            &binding(intent, device)?,
            nf_kernel::intent_digest(&record.intent)?,
            nf_kernel::intent_digest(intent)?,
        )
        .map_err(|_| StoreError::RequestConflict)?;
    Ok(Some(record.status))
}
