//! Closed canonical chat domains; no alias of financial challenge purposes.
use super::{
    Author, Channel, ChatMessage, ChatPolicy, ChatStoreError, HistoryQuery, MAX_TEXT_BYTES, Result,
    SignedMessage,
};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use nf_identity::model::{MembershipState, Scope};
use sha2::{Digest, Sha256};
pub(super) fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn policy_bytes(policy: &ChatPolicy) -> Vec<u8> {
    let mut out = b"NF-CHAT-POLICY-1\0".to_vec();
    out.extend_from_slice(policy.scope.universe.as_bytes());
    out.extend_from_slice(policy.scope.history.as_bytes());
    out.push(1); // Only the general channel is admitted in this profile.
    out.extend_from_slice(&(MAX_TEXT_BYTES as u16).to_be_bytes());
    out.extend_from_slice(&4096u16.to_be_bytes()); // Maximum retained messages.
    out.extend_from_slice(&16384u16.to_be_bytes()); // Maximum local request aliases.
    out.extend_from_slice(&64u16.to_be_bytes()); // Maximum history page.
    out
}
pub fn policy_digest(policy: &ChatPolicy) -> [u8; 32] {
    hash(&policy_bytes(policy))
}
pub fn message_bytes(message: &ChatMessage) -> Result<Vec<u8>> {
    if message.message == [0; 16] || message.sequence == 0 || message.text.is_empty() {
        return Err(ChatStoreError::Malformed);
    }
    if message.text.len() > MAX_TEXT_BYTES {
        return Err(ChatStoreError::Limit);
    }
    let mut out = b"NF-CHAT-MESSAGE-1\0".to_vec();
    out.extend_from_slice(message.scope.universe.as_bytes());
    out.extend_from_slice(message.scope.history.as_bytes());
    out.push(1);
    author(&mut out, message.author);
    out.extend_from_slice(&message.message);
    out.extend_from_slice(&message.sequence.to_be_bytes());
    out.extend_from_slice(&(message.text.len() as u16).to_be_bytes());
    out.extend_from_slice(message.text.as_bytes());
    Ok(out)
}
pub fn message_digest(message: &ChatMessage) -> Result<[u8; 32]> {
    Ok(hash(&message_bytes(message)?))
}
pub(super) fn verify_message(message: &SignedMessage, current: &MembershipState) -> Result<()> {
    if message.message.scope != current.scope {
        return Err(ChatStoreError::Scope);
    }
    let device = current
        .devices
        .get(&message.message.author.device)
        .ok_or(nf_identity::model::IdentityError::UnknownDevice)?;
    if device.account != message.message.author.account {
        return Err(ChatStoreError::Signature);
    }
    nf_contract::signatures::verify_digest(
        &device.key,
        &message_digest(&message.message)?,
        &message.signature,
    )
    .map_err(|_| ChatStoreError::Signature)
}
pub(super) fn post_binding(
    policy: &ChatPolicy,
    request: RequestId,
    message: &SignedMessage,
) -> Result<[u8; 32]> {
    if request.as_bytes() == &[0; 16] {
        return Err(ChatStoreError::Malformed);
    }
    if message.message.scope != policy.scope {
        return Err(ChatStoreError::Scope);
    }
    let mut out = b"NF-CHAT-POST-1\0".to_vec();
    out.extend_from_slice(&policy_digest(policy));
    out.extend_from_slice(request.as_bytes());
    out.extend_from_slice(&message_bytes(&message.message)?);
    out.extend_from_slice(&message.signature);
    Ok(hash(&out))
}
pub(super) fn history_binding(policy: &ChatPolicy, query: &HistoryQuery) -> Result<[u8; 32]> {
    if query.request.as_bytes() == &[0; 16] {
        return Err(ChatStoreError::Malformed);
    }
    if !(1..=64).contains(&query.limit) {
        return Err(ChatStoreError::Limit);
    }
    let mut out = b"NF-CHAT-HISTORY-1\0".to_vec();
    out.extend_from_slice(&policy_digest(policy));
    out.extend_from_slice(query.request.as_bytes());
    author(&mut out, query.reader);
    out.push(1);
    out.extend_from_slice(&query.after_cursor.to_be_bytes());
    out.extend_from_slice(&query.limit.to_be_bytes());
    Ok(hash(&out))
}
fn author(out: &mut Vec<u8>, actor: Author) {
    out.extend_from_slice(actor.account.as_bytes());
    out.extend_from_slice(actor.device.as_bytes());
}

/// Strict bounded canonical decoding; arbitrary trailing bytes are never admitted.
pub(super) fn decode_message(bytes: &[u8]) -> Result<ChatMessage> {
    if bytes.len() > 2157 {
        return Err(ChatStoreError::Limit);
    }
    let mut at = 0;
    if take(bytes, &mut at, 18)? != b"NF-CHAT-MESSAGE-1\0" {
        return Err(ChatStoreError::Malformed);
    }
    let scope = Scope {
        universe: UniverseId::from_bytes(fixed(bytes, &mut at)?),
        history: HistoryId::from_bytes(fixed(bytes, &mut at)?),
    };
    if take(bytes, &mut at, 1)? != [1] {
        return Err(ChatStoreError::Malformed);
    }
    let author = Author {
        account: AccountId::from_bytes(fixed(bytes, &mut at)?),
        device: DeviceId::from_bytes(fixed(bytes, &mut at)?),
    };
    let message = fixed(bytes, &mut at)?;
    let sequence = u64::from_be_bytes(fixed(bytes, &mut at)?);
    let length = usize::from(u16::from_be_bytes(fixed(bytes, &mut at)?));
    if length == 0 || length > MAX_TEXT_BYTES {
        return Err(ChatStoreError::Limit);
    }
    let text = std::str::from_utf8(take(bytes, &mut at, length)?)
        .map_err(|_| ChatStoreError::Malformed)?
        .to_owned();
    if at != bytes.len() {
        return Err(ChatStoreError::Malformed);
    }
    let decoded = ChatMessage {
        scope,
        channel: Channel::General,
        author,
        message,
        sequence,
        text,
    };
    if message_bytes(&decoded)?.as_slice() != bytes {
        return Err(ChatStoreError::Malformed);
    }
    Ok(decoded)
}
fn take<'a>(bytes: &'a [u8], at: &mut usize, length: usize) -> Result<&'a [u8]> {
    let end = at.checked_add(length).ok_or(ChatStoreError::Malformed)?;
    let slice = bytes.get(*at..end).ok_or(ChatStoreError::Malformed)?;
    *at = end;
    Ok(slice)
}
fn fixed<const N: usize>(bytes: &[u8], at: &mut usize) -> Result<[u8; N]> {
    take(bytes, at, N)?
        .try_into()
        .map_err(|_| ChatStoreError::Malformed)
}

/// Canonical value encoding only. A signature is carried but no sender authority is granted.
pub fn encode_signed_message(signed: &SignedMessage) -> Result<Vec<u8>> {
    let mut encoded = message_bytes(&signed.message)?;
    encoded.extend_from_slice(&signed.signature);
    Ok(encoded)
}
/// Bounded canonical value decoding only; ChatStore remains responsible for authentication.
pub fn decode_signed_message(encoded: &[u8]) -> Result<SignedMessage> {
    if !(174..=2221).contains(&encoded.len()) {
        return Err(ChatStoreError::Limit);
    }
    let split = encoded.len() - 64;
    Ok(SignedMessage {
        message: decode_message(&encoded[..split])?,
        signature: encoded[split..]
            .try_into()
            .map_err(|_| ChatStoreError::Malformed)?,
    })
}
