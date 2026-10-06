use crate::{WireError, generated as g};
use nf_contract::canonical::records::{Record, Value};
use std::boxed::Box;
type Result<T> = std::result::Result<T, WireError>;
pub(crate) fn need<T>(value: &Option<T>) -> Result<&T> {
    value.as_ref().ok_or(WireError::Semantic)
}
fn id(value: &[u8]) -> Result<Value> {
    Ok(Value::Id(
        value.try_into().map_err(|_| WireError::Semantic)?,
    ))
}
fn digest(value: &[u8]) -> Result<Value> {
    Ok(Value::Digest(
        value.try_into().map_err(|_| WireError::Semantic)?,
    ))
}
fn nested(record: Record) -> Value {
    Value::Record(Box::new(record))
}
fn rec(domain: u16, kind: u16, fields: Vec<Value>) -> Record {
    Record {
        domain,
        kind,
        fields,
    }
}
fn sorted<'a>(keys: impl IntoIterator<Item = &'a [u8]>) -> Result<()> {
    let mut last: Option<&[u8]> = None;
    for key in keys {
        if last.is_some_and(|previous| previous >= key) {
            return Err(WireError::Semantic);
        }
        last = Some(key);
    }
    Ok(())
}
fn set(ids: &[u32]) -> Result<Value> {
    if ids.len() > 64 {
        return Err(WireError::Limit);
    }
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(WireError::Semantic);
    }
    Ok(Value::Set(ids.to_vec()))
}
pub(crate) fn required(value: &g::RequiredSemantics) -> Result<Record> {
    let record = rec(
        6,
        2,
        vec![set(&value.capability_ids)?, set(&value.schema_ids)?],
    );
    if value
        .capability_ids
        .iter()
        .chain(&value.schema_ids)
        .any(|id| *id != 1)
    {
        return Err(WireError::Unsupported);
    }
    Ok(record)
}
pub(crate) fn payload(value: &g::SchemaPayload) -> Result<Record> {
    if value.schema_id != 1 {
        return Err(WireError::Unsupported);
    }
    nf_contract::canonical::decode_profile(
        &value.canonical_body,
        nf_contract::canonical::Limits::default(),
    )
    .map_err(|_| WireError::Semantic)?;
    Ok(rec(
        6,
        1,
        vec![
            Value::U32(value.schema_id),
            Value::Bytes(value.canonical_body.clone()),
        ],
    ))
}
fn revisions(values: &[g::AggregateVersion]) -> Result<Value> {
    let mut entries = Vec::with_capacity(values.len());
    for value in values {
        entries.push((
            need(&value.aggregate_id)?.value.as_slice(),
            need(&value.revision)?.value,
        ));
    }
    sorted(entries.iter().map(|(key, _)| *key))?;
    Ok(Value::IdMap(
        entries
            .into_iter()
            .map(|(key, value)| Ok((key.try_into().map_err(|_| WireError::Semantic)?, value)))
            .collect::<Result<_>>()?,
    ))
}
fn aggregate(value: &g::AggregateVersion) -> Result<Record> {
    Ok(rec(
        6,
        3,
        vec![
            id(&need(&value.aggregate_id)?.value)?,
            Value::U64(need(&value.revision)?.value),
        ],
    ))
}
fn module(value: &g::ModuleState) -> Result<Record> {
    if value.state_version != 1 {
        return Err(WireError::Unsupported);
    }
    Ok(rec(
        6,
        4,
        vec![
            id(&need(&value.provider_id)?.value)?,
            Value::U32(value.state_version),
            nested(payload(need(&value.state)?)?),
        ],
    ))
}
fn modules(values: &[g::ModuleState]) -> Result<Value> {
    sorted(values.iter().map(|value| {
        value
            .provider_id
            .as_ref()
            .map_or(&[][..], |id| id.value.as_slice())
    }))?;
    Ok(Value::Records(
        values.iter().map(module).collect::<Result<_>>()?,
    ))
}
fn entity(value: &g::EntityState) -> Result<Record> {
    Ok(rec(
        6,
        5,
        vec![
            id(&need(&value.entity_id)?.value)?,
            nested(aggregate(need(&value.aggregate)?)?),
            nested(payload(need(&value.state)?)?),
        ],
    ))
}
fn schedule(value: &g::ScheduledEvent) -> Result<Record> {
    Ok(rec(
        6,
        6,
        vec![
            id(&need(&value.schedule_id)?.value)?,
            Value::U64(need(&value.due_tick)?.value),
            nested(payload(need(&value.event)?)?),
        ],
    ))
}
fn reservation(value: &g::Reservation) -> Result<Record> {
    Ok(rec(
        6,
        7,
        vec![
            id(&need(&value.operation_id)?.value)?,
            nested(payload(need(&value.state)?)?),
        ],
    ))
}
pub(crate) fn error(value: &g::BoundedError) -> Result<Record> {
    if !(1..=10).contains(&value.code) {
        return Err(WireError::Semantic);
    }
    Ok(rec(
        6,
        10,
        vec![
            Value::U32(value.code as u32),
            Value::Text(value.reason.clone()),
            set(&value.unsupported_capability_ids)?,
            set(&value.unsupported_schema_ids)?,
            Value::Bool(value.retryable),
        ],
    ))
}
pub(crate) fn status(value: &g::OperationStatus) -> Result<Record> {
    use g::operation_status::Outcome;
    let outcome = match &value.outcome {
        None => None,
        Some(Outcome::Success(v)) => Some(Box::new(payload(v)?)),
        Some(Outcome::Error(v)) => Some(Box::new(error(v)?)),
    };
    Ok(rec(
        6,
        8,
        vec![
            id(&need(&value.request_id)?.value)?,
            id(&need(&value.operation_id)?.value)?,
            id(&need(&value.history_id)?.value)?,
            digest(&need(&value.request_binding_digest)?.value)?,
            Value::U32(value.phase as u32),
            Value::Outcome(outcome),
            Value::OptionalU64(value.committed_event_seq.as_ref().map(|v| v.value)),
        ],
    ))
}
fn statuses(values: &[g::OperationStatus]) -> Result<Value> {
    sorted(values.iter().map(|value| {
        value
            .operation_id
            .as_ref()
            .map_or(&[][..], |id| id.value.as_slice())
    }))?;
    Ok(Value::Records(
        values.iter().map(status).collect::<Result<_>>()?,
    ))
}
fn dedup(value: &g::DeduplicationEntry) -> Result<Record> {
    let status_value = need(&value.status)?;
    if need(&value.request_id)?.value != need(&status_value.request_id)?.value
        || need(&value.binding_digest)?.value != need(&status_value.request_binding_digest)?.value
    {
        return Err(WireError::Semantic);
    }
    Ok(rec(
        6,
        9,
        vec![
            id(&need(&value.request_id)?.value)?,
            digest(&need(&value.binding_digest)?.value)?,
            nested(status(status_value)?),
        ],
    ))
}
pub(crate) fn intent(value: &g::Intent) -> Result<Record> {
    if value.operation_kind != 1 {
        return Err(WireError::Unsupported);
    }
    let principal = need(&value.principal)?;
    let binding = rec(
        1,
        1,
        vec![
            id(&need(&value.request_id)?.value)?,
            id(&need(&principal.account_id)?.value)?,
            id(&need(&principal.device_id)?.value)?,
            id(&need(&value.universe_id)?.value)?,
            id(&need(&value.history_id)?.value)?,
            Value::U32(value.operation_kind),
            digest(&need(&value.payload_digest)?.value)?,
        ],
    );
    Ok(rec(
        1,
        2,
        vec![
            nested(binding),
            revisions(&value.expected_revisions)?,
            nested(payload(need(&value.payload)?)?),
            Value::OptionalU64(value.expires_at_tick),
            nested(required(need(&value.required)?)?),
        ],
    ))
}
pub(crate) fn snapshot(value: &g::WorldSnapshot) -> Result<Record> {
    sorted(value.entities.iter().map(|v| {
        v.entity_id
            .as_ref()
            .map_or(&[][..], |id| id.value.as_slice())
    }))?;
    sorted(value.scheduled_events.iter().map(|v| {
        v.schedule_id
            .as_ref()
            .map_or(&[][..], |id| id.value.as_slice())
    }))?;
    sorted(value.reservations.iter().map(|v| {
        v.operation_id
            .as_ref()
            .map_or(&[][..], |id| id.value.as_slice())
    }))?;
    sorted(value.deduplication_state.iter().map(|v| {
        v.request_id
            .as_ref()
            .map_or(&[][..], |id| id.value.as_slice())
    }))?;
    let histories = value.unresolved_operations.iter().chain(
        value
            .deduplication_state
            .iter()
            .filter_map(|v| v.status.as_ref()),
    );
    for state in histories {
        if need(&state.history_id)?.value != need(&value.history_id)?.value {
            return Err(WireError::Semantic);
        }
    }
    Ok(rec(
        2,
        1,
        vec![
            id(&need(&value.universe_id)?.value)?,
            id(&need(&value.history_id)?.value)?,
            Value::U64(need(&value.event_seq)?.value),
            Value::U64(need(&value.world_tick)?.value),
            digest(&need(&value.ruleset_hash)?.value)?,
            revisions(&value.aggregate_revisions)?,
            Value::Records(value.entities.iter().map(entity).collect::<Result<_>>()?),
            modules(&value.module_state)?,
            Value::Records(
                value
                    .scheduled_events
                    .iter()
                    .map(schedule)
                    .collect::<Result<_>>()?,
            ),
            Value::Records(
                value
                    .reservations
                    .iter()
                    .map(reservation)
                    .collect::<Result<_>>()?,
            ),
            statuses(&value.unresolved_operations)?,
            Value::Records(
                value
                    .deduplication_state
                    .iter()
                    .map(dedup)
                    .collect::<Result<_>>()?,
            ),
            nested(required(need(&value.required)?)?),
        ],
    ))
}
pub(crate) fn proposal(value: &g::Proposal) -> Result<Record> {
    if value.provider_version != 1 {
        return Err(WireError::Unsupported);
    }
    Ok(rec(
        3,
        1,
        vec![
            id(&need(&value.job_id)?.value)?,
            id(&need(&value.provider_id)?.value)?,
            Value::U32(value.provider_version),
            digest(&need(&value.ruleset_hash)?.value)?,
            digest(&need(&value.input_hash)?.value)?,
            Value::U64(need(&value.runtime_session)?.value),
            revisions(&value.read_set)?,
            revisions(&value.write_set)?,
            Value::Records(
                value
                    .proposed_events
                    .iter()
                    .map(payload)
                    .collect::<Result<_>>()?,
            ),
            modules(&value.module_state_delta)?,
            nested(required(need(&value.required)?)?),
        ],
    ))
}
pub(crate) fn batch(value: &g::EventBatch) -> Result<Record> {
    for outcome in &value.request_outcomes {
        if need(&outcome.history_id)?.value != need(&value.history_id)?.value {
            return Err(WireError::Semantic);
        }
    }
    Ok(rec(
        4,
        1,
        vec![
            id(&need(&value.history_id)?.value)?,
            Value::U64(need(&value.event_seq)?.value),
            digest(&need(&value.previous_hash)?.value)?,
            Value::U64(need(&value.authority_term)?.value),
            Value::U64(need(&value.world_tick)?.value),
            digest(&need(&value.ruleset_hash)?.value)?,
            statuses(&value.request_outcomes)?,
            Value::Records(value.events.iter().map(payload).collect::<Result<_>>()?),
            modules(&value.module_state_changes)?,
            digest(&need(&value.state_hash)?.value)?,
            nested(required(need(&value.required)?)?),
        ],
    ))
}
pub(crate) fn check(record: &Record) -> Result<()> {
    nf_contract::canonical::records::encode_record(record)
        .map(|_| ())
        .map_err(|_| WireError::Semantic)
}
