use super::model::{OutboxProfile, OutgoingEntry};
use crate::chat::{
    Author, Channel, ChatPolicy, ChatReceipt, ChatStoreError, Result, SignedMessage,
    codec as chat_codec,
};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use nf_identity::{keys::SecretSeed, model::Scope};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatDeliveryReceipt {
    pub policy_digest: [u8; 32],
    pub scope: Scope,
    pub channel: Channel,
    pub receiver: Author,
    pub receiver_peer_digest: [u8; 32],
    pub original: ChatReceipt,
    pub signed_message_digest: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedChatReceipt {
    pub receipt: ChatDeliveryReceipt,
    pub signature: [u8; 64],
}
/// Internal issuer path: the caller has verified current local authority and durable originals.
pub(in crate::chat) fn sign_delivery(
    policy: &ChatPolicy,
    receiver: Author,
    peer: &[u8],
    original: &ChatReceipt,
    signed: &SignedMessage,
    key: &SecretSeed,
) -> Result<SignedChatReceipt> {
    let receipt = ChatDeliveryReceipt {
        policy_digest: chat_codec::policy_digest(policy),
        scope: policy.scope,
        channel: signed.message.channel,
        receiver,
        receiver_peer_digest: peer_digest(peer),
        original: *original,
        signed_message_digest: signed_message_digest(signed)?,
    };
    let signature = key.sign(&chat_codec::hash(&bytes(&receipt)?));
    Ok(SignedChatReceipt { receipt, signature })
}
pub(super) fn validate(
    profile: &OutboxProfile,
    original: &OutgoingEntry,
    signed: &SignedChatReceipt,
) -> Result<()> {
    let receipt = &signed.receipt;
    let body = bytes(receipt)?;
    // Incoming fields never choose a public key. Verify the immutable retained receiver pin.
    nf_contract::signatures::verify_digest(
        &profile.receiver.key,
        &chat_codec::hash(&body),
        &signed.signature,
    )
    .map_err(|_| ChatStoreError::Signature)?;
    if receipt.scope != profile.policy.scope {
        return Err(ChatStoreError::Scope);
    }
    if receipt.policy_digest != chat_codec::policy_digest(&profile.policy) {
        return Err(ChatStoreError::Policy);
    }
    if receipt.receiver != profile.receiver.actor
        || receipt.receiver_peer_digest != peer_digest(&profile.receiver.peer)
    {
        return Err(ChatStoreError::Signature);
    }
    if receipt.original.original_request != original.original_request
        || receipt.original.message != original.signed.message.message
        || receipt.original.author != original.signed.message.author
        || receipt.original.source_sequence != original.signed.message.sequence
        || receipt.channel != original.signed.message.channel
        || receipt.signed_message_digest != signed_message_digest(&original.signed)?
    {
        return Err(ChatStoreError::Conflict);
    }
    Ok(())
}
pub(super) fn encode(signed: &SignedChatReceipt) -> Result<Vec<u8>> {
    let mut encoded = bytes(&signed.receipt)?;
    encoded.extend_from_slice(&signed.signature);
    Ok(encoded)
}
pub(super) fn decode(encoded: &[u8]) -> Result<SignedChatReceipt> {
    // Exact fixed size precedes decoding; no receipt field controls an allocation.
    if encoded.len() != 323 {
        return Err(ChatStoreError::Malformed);
    }
    let mut at = 0;
    if take(encoded, &mut at, 18)? != b"NF-CHAT-RECEIPT-1\0" {
        return Err(ChatStoreError::Malformed);
    }
    let policy_digest = fixed(encoded, &mut at)?;
    let scope = Scope {
        universe: UniverseId::from_bytes(fixed(encoded, &mut at)?),
        history: HistoryId::from_bytes(fixed(encoded, &mut at)?),
    };
    if take(encoded, &mut at, 1)? != [1] {
        return Err(ChatStoreError::Malformed);
    }
    let receiver = Author {
        account: AccountId::from_bytes(fixed(encoded, &mut at)?),
        device: DeviceId::from_bytes(fixed(encoded, &mut at)?),
    };
    let receiver_peer_digest = fixed(encoded, &mut at)?;
    let original_request = RequestId::from_bytes(fixed(encoded, &mut at)?);
    let message = fixed(encoded, &mut at)?;
    let author = Author {
        account: AccountId::from_bytes(fixed(encoded, &mut at)?),
        device: DeviceId::from_bytes(fixed(encoded, &mut at)?),
    };
    let source_sequence = u64::from_be_bytes(fixed(encoded, &mut at)?);
    let receiver_cursor = u64::from_be_bytes(fixed(encoded, &mut at)?);
    let signed_message_digest = fixed(encoded, &mut at)?;
    let signature = fixed(encoded, &mut at)?;
    let signed = SignedChatReceipt {
        receipt: ChatDeliveryReceipt {
            policy_digest,
            scope,
            channel: Channel::General,
            receiver,
            receiver_peer_digest,
            original: ChatReceipt {
                original_request,
                message,
                author,
                source_sequence,
                receiver_cursor,
            },
            signed_message_digest,
        },
        signature,
    };
    if at != encoded.len() || encode(&signed)?.as_slice() != encoded {
        return Err(ChatStoreError::Malformed);
    }
    Ok(signed)
}
fn take<'a>(encoded: &'a [u8], at: &mut usize, length: usize) -> Result<&'a [u8]> {
    let end = at.checked_add(length).ok_or(ChatStoreError::Malformed)?;
    let slice = encoded.get(*at..end).ok_or(ChatStoreError::Malformed)?;
    *at = end;
    Ok(slice)
}
fn fixed<const N: usize>(encoded: &[u8], at: &mut usize) -> Result<[u8; N]> {
    take(encoded, at, N)?
        .try_into()
        .map_err(|_| ChatStoreError::Malformed)
}
fn bytes(receipt: &ChatDeliveryReceipt) -> Result<Vec<u8>> {
    if receipt.original.original_request.as_bytes() == &[0; 16]
        || receipt.original.message == [0; 16]
        || receipt.original.source_sequence == 0
        || receipt.original.receiver_cursor == 0
    {
        return Err(ChatStoreError::Malformed);
    }
    // Fixed259 bytes; no incoming variable peer/reason allocation. Persisted upper bound remains420.
    let mut body = Vec::with_capacity(259);
    body.extend_from_slice(b"NF-CHAT-RECEIPT-1\0");
    body.extend_from_slice(&receipt.policy_digest);
    body.extend_from_slice(receipt.scope.universe.as_bytes());
    body.extend_from_slice(receipt.scope.history.as_bytes());
    match receipt.channel {
        Channel::General => body.push(1),
    }
    body.extend_from_slice(receipt.receiver.account.as_bytes());
    body.extend_from_slice(receipt.receiver.device.as_bytes());
    body.extend_from_slice(&receipt.receiver_peer_digest);
    body.extend_from_slice(receipt.original.original_request.as_bytes());
    body.extend_from_slice(&receipt.original.message);
    body.extend_from_slice(receipt.original.author.account.as_bytes());
    body.extend_from_slice(receipt.original.author.device.as_bytes());
    body.extend_from_slice(&receipt.original.source_sequence.to_be_bytes());
    body.extend_from_slice(&receipt.original.receiver_cursor.to_be_bytes());
    body.extend_from_slice(&receipt.signed_message_digest);
    Ok(body)
}
fn peer_digest(peer: &[u8]) -> [u8; 32] {
    // Full1..128-byte peer was validated and retained by trusted local OutboxProfile selection.
    let mut body = b"NF-CHAT-RECEIVER-PEER-1\0".to_vec();
    body.extend_from_slice(&(peer.len() as u16).to_be_bytes());
    body.extend_from_slice(peer);
    chat_codec::hash(&body)
}
fn signed_message_digest(signed: &SignedMessage) -> Result<[u8; 32]> {
    let mut body = b"NF-CHAT-SIGNED-MESSAGE-1\0".to_vec();
    body.extend_from_slice(&chat_codec::message_bytes(&signed.message)?);
    body.extend_from_slice(&signed.signature);
    Ok(chat_codec::hash(&body))
}

impl SignedChatReceipt {
    /// Canonical value encoding only; it does not establish remote delivery authority.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>> {
        encode(self)
    }
    /// Exact323-byte canonical shape only. ClientOutbox verifies retained receiver authority.
    pub fn from_canonical_bytes(encoded: &[u8]) -> Result<Self> {
        decode(encoded)
    }
}
