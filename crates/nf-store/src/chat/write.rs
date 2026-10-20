use super::{ChatPolicy, ChatReceipt, ChatStoreError, Result, SignedMessage, codec, ledger};
use nf_contract::identity::RequestId;
use rusqlite::{Connection, params};
// Pure classification after fresh current authorization; no SQL before rate planning.
pub(super) enum PreparedPost {
    Exact(ChatReceipt),
    Alias {
        receipt: ChatReceipt,
        binding: [u8; 32],
    },
    Original {
        receipt: ChatReceipt,
        binding: [u8; 32],
        body: Vec<u8>,
    },
}
impl PreparedPost {
    pub fn is_fresh(&self) -> bool {
        !matches!(self, Self::Exact(_))
    }
}

pub(super) fn prepare(
    policy: &ChatPolicy,
    state: &ledger::State,
    request: RequestId,
    signed: &SignedMessage,
) -> Result<PreparedPost> {
    let binding = codec::post_binding(policy, request, signed)?;
    if let Some(previous) = state.requests.get(&request) {
        if previous.binding != binding || previous.message != signed.message.message {
            return Err(ChatStoreError::Conflict);
        }
        return Ok(PreparedPost::Exact(
            state
                .messages
                .get(&previous.message)
                .ok_or(ChatStoreError::Corrupt)?
                .receipt,
        ));
    }
    if let Some(entry) = state.messages.get(&signed.message.message) {
        if entry.signed != *signed {
            return Err(ChatStoreError::Conflict);
        }
        if state.requests.len() >= ledger::MAX_REQUESTS {
            return Err(ChatStoreError::Limit);
        }
        return Ok(PreparedPost::Alias {
            receipt: entry.receipt,
            binding,
        });
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
    Ok(PreparedPost::Original {
        receipt,
        binding,
        body,
    })
}
// Applies only the already checked plan inside the same transaction as its account debit.
pub(super) fn apply(
    connection: &Connection,
    state: &ledger::State,
    request: RequestId,
    signed: &SignedMessage,
    prepared: &PreparedPost,
) -> Result<ChatReceipt> {
    let (receipt, binding, body) = match prepared {
        PreparedPost::Exact(receipt) => return Ok(*receipt),
        PreparedPost::Alias { receipt, binding } => {
            insert_request(connection, request, *binding, receipt.message)?;
            return Ok(*receipt);
        }
        PreparedPost::Original {
            receipt,
            binding,
            body,
        } => (*receipt, *binding, body),
    };
    let cursor = receipt.receiver_cursor;
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
    #[cfg(test)]
    super::rate_crash_controls::cut("post-after-original");
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
    #[cfg(test)]
    super::rate_crash_controls::cut("post-after-request");
    Ok(())
}
