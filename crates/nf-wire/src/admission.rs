use crate::{WireError, generated as g, records};
use records::need;
type Result<T> = std::result::Result<T, WireError>;
fn equal(left: &g::RequiredSemantics, right: &g::RequiredSemantics) -> Result<()> {
    records::required(left)?;
    records::required(right)?;
    if left != right {
        return Err(WireError::Semantic);
    }
    Ok(())
}
pub(crate) fn limits(value: &g::ResourceLimits) -> Result<()> {
    let actual = [
        value.control_frame_bytes as u64,
        value.chunk_bytes as u64,
        value.inflight_bytes as u64,
        value.inflight_items as u64,
        value.decoded_bytes as u64,
        value.collection_items as u64,
        value.nesting_depth as u64,
        value.transfer_bytes,
    ];
    let hard = [
        1_048_576, 262_144, 16_777_216, 256, 1_048_576, 4096, 32, 67_108_864,
    ];
    if actual.iter().zip(hard).any(|(n, cap)| *n == 0 || *n > cap) {
        return Err(WireError::Limit);
    }
    if value.chunk_bytes > value.control_frame_bytes
        || value.control_frame_bytes > value.decoded_bytes
        || value.control_frame_bytes > value.inflight_bytes
        || value.chunk_bytes as u64 > value.transfer_bytes
    {
        return Err(WireError::Semantic);
    }
    Ok(())
}
pub(crate) fn control(value: &g::ControlEnvelope) -> Result<()> {
    use g::{control_envelope::Body, handshake_result::Result as HandshakeResult};
    if value.protocol_version != 1 {
        return Err(WireError::Unsupported);
    }
    let required = need(&value.required)?;
    // New Chat bodies have their own closed registry. Legacy canonical records stay schema1.
    if matches!(
        need(&value.body)?,
        Body::QueryChatOutgoing(_) | Body::ChatOutgoingStatus(_)
    ) {
        crate::chat_status::required(required)?;
    } else if matches!(
        need(&value.body)?,
        Body::EnqueueChat(_) | Body::ChatEnqueueResult(_)
    ) {
        crate::chat_enqueue::required(required)?;
    } else {
        records::required(required)?;
    }
    let runtime = need(&value.runtime_session)?.value;
    match need(&value.body)? {
        Body::Handshake(v) => {
            let range = need(&v.protocols)?;
            if range.minimum == 0 || range.minimum > range.maximum {
                return Err(WireError::Semantic);
            }
            if range.minimum > 1 || range.maximum < 1 {
                return Err(WireError::Unsupported);
            }
            equal(required, need(&v.required)?)?;
            if need(&v.runtime_session)?.value != runtime {
                return Err(WireError::Semantic);
            }
            if v.optional_capability_ids.len() > 64 {
                return Err(WireError::Limit);
            }
            if v.optional_capability_ids.contains(&0)
                || v.optional_capability_ids.windows(2).any(|p| p[0] >= p[1])
            {
                return Err(WireError::Semantic);
            }
            limits(need(&v.limits)?)?;
        }
        Body::HandshakeResult(v) => match need(&v.result)? {
            HandshakeResult::Accepted(v) => {
                if v.protocol_version != 1 {
                    return Err(WireError::Unsupported);
                }
                equal(required, need(&v.semantics)?)?;
                if need(&v.runtime_session)?.value != runtime {
                    return Err(WireError::Semantic);
                }
                limits(need(&v.limits)?)?;
            }
            HandshakeResult::Rejected(v) => records::check(&records::error(v)?)?,
        },
        Body::Intent(v) => {
            equal(required, need(&v.required)?)?;
            records::check(&records::intent(v)?)?;
        }
        Body::Proposal(v) => {
            equal(required, need(&v.required)?)?;
            if need(&v.runtime_session)?.value != runtime {
                return Err(WireError::Semantic);
            }
            records::check(&records::proposal(v)?)?;
        }
        Body::EventBatch(v) => {
            equal(required, need(&v.required)?)?;
            records::check(&records::batch(v)?)?;
        }
        Body::OperationStatus(v) => records::check(&records::status(v)?)?,
        Body::Error(v) => records::check(&records::error(v)?)?,
        Body::QueryOperation(_) => {}
        Body::QueryChatOutgoing(v) => crate::chat_status::validate_query_chat_outgoing(v)?,
        Body::ChatOutgoingStatus(v) => crate::chat_status::validate_chat_outgoing_status(v)?,
        Body::EnqueueChat(v) => crate::chat_enqueue::validate_enqueue_chat(v)?,
        Body::ChatEnqueueResult(v) => crate::chat_enqueue::validate_chat_enqueue_result(v)?,
        // No registered cancellation payload or lifecycle yet. Fail closed.
        Body::CancelOperation(_) => return Err(WireError::Unsupported),
    }
    Ok(())
}
pub(crate) fn snapshot(value: &g::WorldSnapshot) -> Result<()> {
    let record = records::snapshot(value)?;
    let expected =
        nf_contract::canonical::records::record_digest(&record).map_err(|_| WireError::Semantic)?;
    if need(&value.state_hash)?.value != expected {
        return Err(WireError::Semantic);
    }
    Ok(())
}
pub(crate) fn chunk(value: &g::SnapshotChunk) -> Result<()> {
    if value.chunk_count == 0
        || value.chunk_count > 256
        || value.total_bytes == 0
        || value.total_bytes > 67_108_864
        || value.data.len() > 262_144
    {
        return Err(WireError::Limit);
    }
    if value.chunk_index >= value.chunk_count
        || value.data.is_empty()
        || value.data.len() as u64 > value.total_bytes
        || value.total_bytes > value.chunk_count as u64 * 262_144
    {
        return Err(WireError::Semantic);
    }
    Ok(())
}
