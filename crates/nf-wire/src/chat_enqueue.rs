//! Closed data-only local Chat command profile4; never economic or signature authority.
use crate::{WireError, generated as g};
type Result<T> = std::result::Result<T, WireError>;
pub(crate) fn required(value: &g::RequiredSemantics) -> Result<()> {
    if value.capability_ids != [4] || value.schema_ids != [4] {
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
pub fn validate_enqueue_chat(value: &g::EnqueueChat) -> Result<()> {
    id(value.request_id.as_ref().map(|v| v.value.as_slice()))?;
    let principal = value.principal.as_ref().ok_or(WireError::Semantic)?;
    id(principal.account_id.as_ref().map(|v| v.value.as_slice()))?;
    id(principal.device_id.as_ref().map(|v| v.value.as_slice()))?;
    id(value.universe_id.as_ref().map(|v| v.value.as_slice()))?;
    id(value.history_id.as_ref().map(|v| v.value.as_slice()))?;
    id(value
        .original_request_id
        .as_ref()
        .map(|v| v.value.as_slice()))?;
    if value.channel != 1 {
        return Err(WireError::Unsupported);
    }
    if value.text.is_empty() {
        return Err(WireError::Semantic);
    }
    if value.text.len() > 2048 {
        return Err(WireError::Limit);
    }
    // Preserve the existing IPC canonical text rule; never normalize submitted bytes.
    if !nf_contract::canonical::text::is_canonical_text(&value.text) {
        return Err(WireError::Semantic);
    }
    Ok(())
}
pub fn validate_chat_enqueue_result(value: &g::ChatEnqueueResult) -> Result<()> {
    crate::validate_chat_outgoing_status(value.status.as_ref().ok_or(WireError::Semantic)?)
}
