//! Bounded snapshot verification. Neither history rows nor aliases are trusted caches.
use super::{ChatPolicy, ChatReceipt, ChatStoreError, Result, SignedMessage, codec};
use nf_contract::identity::RequestId;
use nf_identity::model::MembershipState;
use rusqlite::Connection;
use std::collections::BTreeMap;
pub(super) const MAX_MESSAGES: usize = 4096;
pub(super) const MAX_REQUESTS: usize = 16384;
pub(super) type StreamPosition = ([u8; 16], [u8; 16], u64);
pub(super) struct Entry {
    pub signed: SignedMessage,
    pub receipt: ChatReceipt,
}
pub(super) struct Request {
    pub binding: [u8; 32],
    pub message: [u8; 16],
}
pub(super) struct State {
    pub revision: u64,
    pub messages: BTreeMap<[u8; 16], Entry>,
    pub requests: BTreeMap<RequestId, Request>,
    pub streams: BTreeMap<StreamPosition, [u8; 16]>,
}
struct StoredMessage {
    id: Vec<u8>,
    account: Vec<u8>,
    device: Vec<u8>,
    sequence: Vec<u8>,
    cursor: Vec<u8>,
    body: Vec<u8>,
    signature: Vec<u8>,
    original_request: Vec<u8>,
}
fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes.try_into().map_err(|_| ChatStoreError::Corrupt)
}
pub(super) fn position(signed: &SignedMessage) -> StreamPosition {
    (
        *signed.message.author.account.as_bytes(),
        *signed.message.author.device.as_bytes(),
        signed.message.sequence,
    )
}
pub(super) fn verify(
    connection: &Connection,
    policy: &ChatPolicy,
    revision: u64,
    membership: &MembershipState,
) -> Result<State> {
    let mut state = State {
        revision,
        messages: BTreeMap::new(),
        requests: BTreeMap::new(),
        streams: BTreeMap::new(),
    };
    let mut statement = connection.prepare("SELECT message,account,device,sequence,receiver_cursor,body,signature,original_request FROM chat_messages ORDER BY receiver_cursor LIMIT 4097")?;
    let rows = statement.query_map([], |row| {
        Ok(StoredMessage {
            id: crate::persistence::bounded_blob(row, 0, 16)?,
            account: crate::persistence::bounded_blob(row, 1, 16)?,
            device: crate::persistence::bounded_blob(row, 2, 16)?,
            sequence: crate::persistence::bounded_blob(row, 3, 8)?,
            cursor: crate::persistence::bounded_blob(row, 4, 8)?,
            body: crate::persistence::bounded_blob(row, 5, 2176)?,
            signature: crate::persistence::bounded_blob(row, 6, 64)?,
            original_request: crate::persistence::bounded_blob(row, 7, 16)?,
        })
    })?;
    for row in rows {
        if state.messages.len() >= MAX_MESSAGES {
            return Err(ChatStoreError::Corrupt);
        }
        let row = row?;
        let message = codec::decode_message(&row.body).map_err(|_| ChatStoreError::Corrupt)?;
        let signed = SignedMessage {
            message,
            signature: array(&row.signature)?,
        };
        // Current revocation does not erase delivered copies: this verifies retained key/signature, not original PLAYER liveness.
        codec::verify_message(&signed, membership).map_err(|_| ChatStoreError::Corrupt)?;
        if signed.message.scope != policy.scope
            || row.id.as_slice() != signed.message.message
            || row.account.as_slice() != signed.message.author.account.as_bytes()
            || row.device.as_slice() != signed.message.author.device.as_bytes()
            || crate::schema::read_counter(&row.sequence)? != signed.message.sequence
        {
            return Err(ChatStoreError::Corrupt);
        }
        let cursor = crate::schema::read_counter(&row.cursor)?;
        if cursor
            != (state.messages.len() as u64)
                .checked_add(1)
                .ok_or(ChatStoreError::Corrupt)?
        {
            return Err(ChatStoreError::Corrupt);
        }
        let original_request = RequestId::from_bytes(array(&row.original_request)?);
        if original_request.as_bytes() == &[0; 16] {
            return Err(ChatStoreError::Corrupt);
        }
        let receipt = ChatReceipt {
            message: signed.message.message,
            author: signed.message.author,
            source_sequence: signed.message.sequence,
            receiver_cursor: cursor,
            original_request,
        };
        if state
            .streams
            .insert(position(&signed), receipt.message)
            .is_some()
            || state
                .messages
                .insert(receipt.message, Entry { signed, receipt })
                .is_some()
        {
            return Err(ChatStoreError::Corrupt);
        }
    }
    if state.revision != state.messages.len() as u64 {
        return Err(ChatStoreError::Corrupt);
    }
    let mut statement = connection.prepare(
        "SELECT request,binding_digest,message FROM chat_requests ORDER BY request LIMIT 16385",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            crate::persistence::bounded_blob(row, 0, 16)?,
            crate::persistence::bounded_blob(row, 1, 32)?,
            crate::persistence::bounded_blob(row, 2, 16)?,
        ))
    })?;
    for row in rows {
        if state.requests.len() >= MAX_REQUESTS {
            return Err(ChatStoreError::Corrupt);
        }
        let (request, binding, message) = row?;
        let request = RequestId::from_bytes(array(&request)?);
        let message = array(&message)?;
        let entry = state
            .messages
            .get(&message)
            .ok_or(ChatStoreError::Corrupt)?;
        let expected = codec::post_binding(policy, request, &entry.signed)
            .map_err(|_| ChatStoreError::Corrupt)?;
        if binding.as_slice() != expected.as_slice() {
            return Err(ChatStoreError::Corrupt);
        }
        if state
            .requests
            .insert(
                request,
                Request {
                    binding: expected,
                    message,
                },
            )
            .is_some()
        {
            return Err(ChatStoreError::Corrupt);
        }
    }
    for entry in state.messages.values() {
        let original = state
            .requests
            .get(&entry.receipt.original_request)
            .ok_or(ChatStoreError::Corrupt)?;
        if original.message != entry.receipt.message {
            return Err(ChatStoreError::Corrupt);
        }
    }
    Ok(state)
}
