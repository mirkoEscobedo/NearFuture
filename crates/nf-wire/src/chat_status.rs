//! Data-only Chat IPC profile3. This never authenticates a message or grants financial authority.
use crate::{WireError, generated as g};
type Result<T> = std::result::Result<T, WireError>;
pub(crate) fn required(value: &g::RequiredSemantics) -> Result<()> {
    if value.capability_ids != [3] || value.schema_ids != [3] {
        return Err(WireError::Unsupported);
    }
    Ok(())
}
fn id(value: Option<&[u8]>) -> Result<()> {
    let value = value.ok_or(WireError::Semantic)?;
    if value.len() != 16 || value == [0; 16] {
        return Err(WireError::Semantic);
    }
    Ok(())
}
fn common(
    request: Option<&g::RequestId>,
    principal: Option<&g::Principal>,
    universe: Option<&g::UniverseId>,
    history: Option<&g::HistoryId>,
    original: Option<&g::RequestId>,
    message: Option<&g::ChatMessageId>,
) -> Result<()> {
    id(request.map(|v| v.value.as_slice()))?;
    let principal = principal.ok_or(WireError::Semantic)?;
    id(principal.account_id.as_ref().map(|v| v.value.as_slice()))?;
    id(principal.device_id.as_ref().map(|v| v.value.as_slice()))?;
    id(universe.map(|v| v.value.as_slice()))?;
    id(history.map(|v| v.value.as_slice()))?;
    id(original.map(|v| v.value.as_slice()))?;
    id(message.map(|v| v.value.as_slice()))
}
pub fn validate_query_chat_outgoing(value: &g::QueryChatOutgoing) -> Result<()> {
    common(
        value.request_id.as_ref(),
        value.principal.as_ref(),
        value.universe_id.as_ref(),
        value.history_id.as_ref(),
        value.original_request_id.as_ref(),
        value.message_id.as_ref(),
    )
}
pub fn validate_chat_outgoing_status(value: &g::ChatOutgoingStatus) -> Result<()> {
    common(
        value.request_id.as_ref(),
        value.principal.as_ref(),
        value.universe_id.as_ref(),
        value.history_id.as_ref(),
        value.original_request_id.as_ref(),
        value.message_id.as_ref(),
    )?;
    if value.source_sequence == 0 || !(1..=8192).contains(&value.outbox_revision) {
        return Err(WireError::Semantic);
    }
    if !(174..=2221).contains(&value.signed_message.len())
        || !value.signed_message.starts_with(b"NF-CHAT-MESSAGE-1\0")
    {
        return Err(WireError::Semantic);
    }
    match g::ChatOutgoingPhase::try_from(value.phase).map_err(|_| WireError::Unsupported)? {
        g::ChatOutgoingPhase::Pending if value.receiver_receipt.is_empty() => Ok(()),
        g::ChatOutgoingPhase::Delivered
            if value.receiver_receipt.len() == 323
                && value.receiver_receipt.starts_with(b"NF-CHAT-RECEIPT-1\0") =>
        {
            Ok(())
        }
        _ => Err(WireError::Semantic),
    }
}
