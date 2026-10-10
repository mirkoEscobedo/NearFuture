use super::{ChatPolicy, ChatReceipt, ChatStoreError, Result, SignedMessage, codec, ledger};
use nf_contract::identity::RequestId;
use rusqlite::{Connection, params};
// Called only after fresh current Chat authorization in the owner's Immediate transaction.
pub(super) fn post(
    connection: &Connection,
    policy: &ChatPolicy,
    state: &ledger::State,
    request: RequestId,
    signed: &SignedMessage,
) -> Result<ChatReceipt> {
    let binding = codec::post_binding(policy, request, signed)?;
    if let Some(previous) = state.requests.get(&request) {
        if previous.binding != binding || previous.message != signed.message.message {
            return Err(ChatStoreError::Conflict);
        }
        return Ok(state
            .messages
            .get(&previous.message)
            .ok_or(ChatStoreError::Corrupt)?
            .receipt);
    }
    if let Some(entry) = state.messages.get(&signed.message.message) {
        if entry.signed != *signed {
            return Err(ChatStoreError::Conflict);
        }
        if state.requests.len() >= ledger::MAX_REQUESTS {
            return Err(ChatStoreError::Limit);
        }
        insert_request(connection, request, binding, entry.receipt.message)?;
        return Ok(entry.receipt);
    }
    if state.streams.contains_key(&ledger::position(signed)) {
        return Err(ChatStoreError::Conflict);
    }
    if state.messages.len() >= ledger::MAX_MESSAGES || state.requests.len() >= ledger::MAX_REQUESTS
    {
        return Err(ChatStoreError::Limit);
    }
    let cursor = state.revision.checked_add(1).ok_or(ChatStoreError::Limit)?;
    let body = codec::message_bytes(&signed.message)?;
    let receipt = ChatReceipt {
        message: signed.message.message,
        author: signed.message.author,
        source_sequence: signed.message.sequence,
        receiver_cursor: cursor,
        original_request: request,
    };
    // Complete bounded canonical body/receipt/capacity checks precede all SQL.
    let changed = connection.execute(
        "INSERT INTO chat_messages VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            receipt.message.as_slice(),
            receipt.author.account.as_bytes(),
            receipt.author.device.as_bytes(),
            crate::schema::counter(receipt.source_sequence),
            crate::schema::counter(cursor),
            body,
            signed.signature.as_slice(),
            request.as_bytes()
        ],
    )?;
    if changed != 1 {
        return Err(ChatStoreError::Corrupt);
    }
    insert_request(connection, request, binding, receipt.message)?;
    if connection.execute(
        "UPDATE chat_meta SET revision=?1 WHERE singleton=1 AND revision=?2",
        params![
            crate::schema::counter(cursor),
            crate::schema::counter(state.revision)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(receipt)
}
fn insert_request(
    connection: &Connection,
    request: RequestId,
    binding: [u8; 32],
    message: [u8; 16],
) -> Result<()> {
    if connection.execute(
        "INSERT INTO chat_requests VALUES(?1,?2,?3)",
        params![request.as_bytes(), binding.as_slice(), message.as_slice()],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(())
}
