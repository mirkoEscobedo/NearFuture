//! Called only after full bounded snapshot and genuine retained sender/receiver validation.
use super::{
    model::{OutgoingEntry, OutgoingState},
    receipt::{self, SignedChatReceipt},
    schema::State,
};
use crate::chat::{ChatStoreError, Result, SignedMessage, codec as chat_codec};
use nf_contract::identity::RequestId;
use rusqlite::{Connection, params};

pub(super) fn enqueue(
    connection: &Connection,
    state: &State,
    request: RequestId,
    signed: &SignedMessage,
) -> Result<OutgoingEntry> {
    if let Some(original) = state.entries.get(&signed.message.message) {
        if original.original_request != request || original.signed != *signed {
            return Err(ChatStoreError::Conflict);
        }
        return Ok(original.clone());
    }
    if state.entries.values().any(|entry| {
        entry.original_request == request
            || (entry.signed.message.author == signed.message.author
                && entry.signed.message.sequence == signed.message.sequence)
    }) {
        return Err(ChatStoreError::Conflict);
    }
    if state.entries.len() >= 4096 {
        return Err(ChatStoreError::Limit);
    }
    let revision = state.revision.checked_add(1).ok_or(ChatStoreError::Limit)?;
    let body = chat_codec::message_bytes(&signed.message)?;
    let entry = OutgoingEntry {
        original_request: request,
        signed: signed.clone(),
        state: OutgoingState::Pending,
    };
    // All canonical/conflict/capacity calculations precede the first SQL effect.
    if connection.execute(
        "INSERT INTO outbox_messages VALUES(?1,?2,?3,?4,?5,?6,?7,1,NULL)",
        params![
            signed.message.message.as_slice(),
            signed.message.author.account.as_bytes(),
            signed.message.author.device.as_bytes(),
            crate::schema::counter(signed.message.sequence),
            request.as_bytes(),
            body,
            signed.signature.as_slice()
        ],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    if connection.execute(
        "UPDATE outbox_meta SET revision=?1 WHERE singleton=1 AND revision=?2",
        params![
            crate::schema::counter(revision),
            crate::schema::counter(state.revision)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(entry)
}

pub(super) fn acknowledge(
    connection: &Connection,
    state: &State,
    signed: &SignedChatReceipt,
) -> Result<OutgoingEntry> {
    let original = state
        .entries
        .get(&signed.receipt.original.message)
        .ok_or(ChatStoreError::Conflict)?;
    match &original.state {
        OutgoingState::Delivered(retained) if retained.as_ref() == signed => {
            return Ok(original.clone());
        }
        OutgoingState::Delivered(_) => return Err(ChatStoreError::Conflict),
        OutgoingState::Pending => {}
    }
    let revision = state.revision.checked_add(1).ok_or(ChatStoreError::Limit)?;
    let encoded = receipt::encode(signed)?;
    let mut entry = original.clone();
    entry.state = OutgoingState::Delivered(Box::new(signed.clone()));
    // Original request/body/signature remain immutable; only the pending phase may transition.
    if connection.execute(
        "UPDATE outbox_messages SET phase=2,receipt=?1 WHERE message=?2 AND phase=1 AND receipt IS NULL",
        params![encoded, signed.receipt.original.message.as_slice()],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    if connection.execute(
        "UPDATE outbox_meta SET revision=?1 WHERE singleton=1 AND revision=?2",
        params![
            crate::schema::counter(revision),
            crate::schema::counter(state.revision)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(entry)
}
