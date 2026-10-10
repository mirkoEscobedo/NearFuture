use super::model::{OutboxProfile, RetainedKey};
use crate::chat::{ChatStoreError, Result, SignedMessage, codec as chat_codec};
use nf_contract::identity::RequestId;

pub(super) fn profile_bytes(profile: &OutboxProfile) -> Vec<u8> {
    let mut body = b"NF-CHAT-OUTBOX-PROFILE-1\0".to_vec();
    body.extend_from_slice(&chat_codec::policy_digest(&profile.policy));
    body.extend_from_slice(profile.policy.scope.universe.as_bytes());
    body.extend_from_slice(profile.policy.scope.history.as_bytes());
    retained(&mut body, &profile.sender);
    retained(&mut body, &profile.receiver);
    body.extend_from_slice(&4096u16.to_be_bytes());
    body
}
fn retained(body: &mut Vec<u8>, key: &RetainedKey) {
    body.extend_from_slice(key.actor.account.as_bytes());
    body.extend_from_slice(key.actor.device.as_bytes());
    body.extend_from_slice(&key.key);
    body.extend_from_slice(&(key.peer.len() as u16).to_be_bytes());
    body.extend_from_slice(&key.peer);
}
pub(super) fn validate(
    profile: &OutboxProfile,
    request: RequestId,
    signed: &SignedMessage,
) -> Result<()> {
    if request.as_bytes() == &[0; 16] {
        return Err(ChatStoreError::Malformed);
    }
    if signed.message.scope != profile.policy.scope {
        return Err(ChatStoreError::Scope);
    }
    if signed.message.author != profile.sender.actor {
        return Err(ChatStoreError::Signature);
    }
    nf_contract::signatures::verify_digest(
        &profile.sender.key,
        &chat_codec::message_digest(&signed.message)?,
        &signed.signature,
    )
    .map_err(|_| ChatStoreError::Signature)
}
