use super::{error::Result, model::*, state::State};
use crate::StoreError;
use nf_contract::identity::*;
pub(crate) fn claim(old: &State, a: MiniatureAuthority) -> Result<State> {
    let term = old.authority.map_or(AuthorityTerm(0), |a| a.term);
    if term.checked_next() != Some(a.term)
        || a.session.0 == 0
        || old.authority.is_some_and(|prior| {
            prior.session == a.session || prior.membership_revision > a.membership_revision
        })
    {
        return Err(StoreError::InvalidTransition.into());
    }
    let mut next = old.clone();
    next.authority = Some(a);
    next.validate()?;
    Ok(next)
}
pub(crate) fn claim_action(a: MiniatureAuthority) -> Vec<u8> {
    let mut b = vec![2];
    b.extend_from_slice(&a.term.0.to_le_bytes());
    b.extend_from_slice(&a.session.0.to_le_bytes());
    b.extend_from_slice(a.account.as_bytes());
    b.extend_from_slice(a.device.as_bytes());
    b.extend_from_slice(&a.membership_revision.to_le_bytes());
    b
}
pub(crate) fn replay(old: &State, kind: u32, action: &[u8]) -> Result<State> {
    match kind {
        1 if action.first() == Some(&2) && action.len() == 57 => {
            let array = |start: usize| {
                action[start..start + 8]
                    .try_into()
                    .map_err(|_| StoreError::Corrupt)
            };
            let a = MiniatureAuthority {
                term: AuthorityTerm(u64::from_le_bytes(array(1)?)),
                session: RuntimeSession(u64::from_le_bytes(array(9)?)),
                account: AccountId::from_slice(&action[17..33]).map_err(|_| StoreError::Corrupt)?,
                device: DeviceId::from_slice(&action[33..49]).map_err(|_| StoreError::Corrupt)?,
                membership_revision: u64::from_le_bytes(array(49)?),
            };
            claim(old, a)
        }
        1 if matches!(action.first(), Some(1 | 3)) => {
            if action.len() < 5 {
                return Err(StoreError::Corrupt.into());
            }
            let length =
                u32::from_le_bytes(action[1..5].try_into().map_err(|_| StoreError::Corrupt)?)
                    as usize;
            if length > 1_048_576 {
                return Err(StoreError::Limit.into());
            }
            if length != action.len() - 5 {
                return Err(StoreError::Corrupt.into());
            }
            let frontier = nf_kernel::miniature::decode_miniature_frontier(&action[5..])?;
            if action[0] == 1 {
                prepare(old, &frontier)
            } else {
                resume(old, &frontier)
            }
        }
        2 => {
            let batch = nf_kernel::miniature::decode_miniature_batch(action, &old.world)?;
            commit(old, &batch)
        }
        3 if action.len() == 16 => {
            let operation = OperationId::from_slice(action).map_err(|_| StoreError::Corrupt)?;
            let mut next = old.clone();
            if next.outbox.remove(&operation).is_none() {
                return Err(StoreError::Corrupt.into());
            }
            next.validate()?;
            Ok(next)
        }
        _ => Err(StoreError::UnsupportedSchema.into()),
    }
}

pub(crate) fn prepare(
    old: &State,
    frontier: &nf_kernel::miniature::MiniatureFrontier,
) -> Result<State> {
    use nf_kernel::miniature::*;
    let a = old.authority.ok_or(StoreError::InvalidTransition)?;
    if frontier.snapshot() != &old.world || frontier.authority() != a.context() {
        return Err(StoreError::InvalidTransition.into());
    }
    for i in frontier.intents() {
        if let Some(record) = old.requests.get(&i.request)
            && record.intent != *i
        {
            return Err(StoreError::RequestConflict.into());
        }
    }
    if let Some(existing) = &old.pending {
        if existing == frontier {
            return Ok(old.clone());
        }
        return Err(StoreError::InvalidTransition.into());
    }
    if old
        .requests
        .len()
        .checked_add(frontier.intents().len())
        .is_none_or(|n| n > 4096)
    {
        return Err(StoreError::Limit.into());
    }
    if old
        .outbox
        .len()
        .checked_add(frontier.intents().len())
        .is_none_or(|n| n > 256)
    {
        return Err(StoreError::Backpressure.into());
    }
    let mut next = old.clone();
    next.pending = Some(frontier.clone());
    for i in frontier.intents() {
        if old.requests.contains_key(&i.request) {
            return Err(StoreError::RequestConflict.into());
        }
        next.requests.insert(
            i.request,
            MiniatureBoundRequest {
                intent: i.clone(),
                status: MiniatureRequestStatus::Pending {
                    operation: i.operation,
                },
                binding_digest: miniature_request_binding(i)?.digest(),
            },
        );
    }
    next.validate()?;
    Ok(next)
}
pub(crate) fn frontier_action(
    tag: u8,
    frontier: &nf_kernel::miniature::MiniatureFrontier,
) -> Result<Vec<u8>> {
    let body = nf_kernel::miniature::encode_miniature_frontier(frontier)?;
    if body.len() > 1_048_576 - 5 {
        return Err(StoreError::Limit.into());
    }
    let mut out = Vec::with_capacity(body.len() + 5);
    out.push(tag);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

pub(crate) fn commit(old: &State, batch: &nf_kernel::miniature::MiniatureBatch) -> Result<State> {
    use nf_kernel::miniature::*;
    let a = old.authority.ok_or(StoreError::InvalidTransition)?;
    let frontier = old.pending.as_ref().ok_or(StoreError::InvalidTransition)?;
    if batch.authority() != a.context() || batch.membership_revision() < a.membership_revision {
        return Err(StoreError::InvalidTransition.into());
    }
    let expected = match batch.disposition() {
        MiniatureDisposition::AdvanceTick => settle_miniature(&old.world, frontier, a.context())?,
        MiniatureDisposition::CancelPending => {
            let reason = batch
                .outcomes()
                .first()
                .map_or(Some(MiniatureRejection::Cancelled), |o| o.rejection)
                .ok_or(StoreError::InvalidTransition)?;
            cancel_miniature(
                &old.world,
                frontier,
                a.context(),
                batch.membership_revision(),
                reason,
            )?
        }
    };
    if expected != *batch {
        return Err(StoreError::InvalidTransition.into());
    }
    if old
        .outbox
        .len()
        .checked_add(batch.outcomes().len())
        .is_none_or(|n| n > 256)
    {
        return Err(StoreError::Backpressure.into());
    }
    let world = replay_miniature(&old.world, batch)?;
    let digest = crate::schema::hash(&encode_miniature_batch(batch)?);
    let mut next = old.clone();
    next.world = world;
    next.pending = None;
    next.authority = Some(MiniatureAuthority {
        membership_revision: batch.membership_revision(),
        ..a
    });
    for i in batch.intents() {
        let record = next
            .requests
            .get_mut(&i.request)
            .ok_or(StoreError::InvalidTransition)?;
        let outcome = batch
            .outcomes()
            .iter()
            .find(|o| o.operation == i.operation && o.job == i.job)
            .ok_or(StoreError::InvalidTransition)?;
        if record.intent != *i || !matches!(record.status, MiniatureRequestStatus::Pending { .. }) {
            return Err(StoreError::InvalidTransition.into());
        }
        record.status = MiniatureRequestStatus::Committed {
            operation: i.operation,
            sequence: batch.event_sequence(),
            rejection: outcome.rejection,
        };
        if next
            .outbox
            .insert(
                i.operation,
                MiniatureOutbox {
                    operation: i.operation,
                    sequence: batch.event_sequence(),
                    batch_digest: digest,
                    rejection: outcome.rejection,
                },
            )
            .is_some()
        {
            return Err(StoreError::InvalidTransition.into());
        }
    }
    next.validate()?;
    Ok(next)
}

pub(crate) fn resume(
    old: &State,
    frontier: &nf_kernel::miniature::MiniatureFrontier,
) -> Result<State> {
    let prior = old.pending.as_ref().ok_or(StoreError::InvalidTransition)?;
    let a = old.authority.ok_or(StoreError::InvalidTransition)?;
    if frontier.snapshot() != &old.world
        || frontier.authority() != a.context()
        || frontier.intents() != prior.intents()
        || frontier.membership_revision() < prior.membership_revision()
        || frontier.membership_revision() < a.membership_revision
        || frontier.input_hash() != prior.input_hash()
        || frontier.reservation() != prior.reservation()
    {
        return Err(StoreError::InvalidTransition.into());
    }
    let mut next = old.clone();
    next.pending = Some(frontier.clone());
    next.authority = Some(MiniatureAuthority {
        membership_revision: frontier.membership_revision(),
        ..a
    });
    next.validate()?;
    Ok(next)
}
