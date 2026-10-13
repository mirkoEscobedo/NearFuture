//! Trusted local receipt issuance; possession of a local seed never enrolls a remote key.
use super::{Author, ChatPolicy, ChatStoreError, Result, codec, ledger, outbox};
use nf_contract::identity::RequestId;
use nf_identity::{
    keys::SecretSeed,
    model::{IdentityError, MembershipState, Roles},
};

/// Ephemeral LOCAL supervisor selection. The seed remains borrowed and never enters SQLite.
/// Scope, current PLAYER membership, account/device, full peer and seed-derived key are rechecked
/// against this ChatStore's durable membership on every call. This is not a wire enrollment port.
pub struct LocalReceiptIssuer<'a> {
    pub policy: ChatPolicy,
    pub receiver: Author,
    pub peer: &'a [u8],
    pub device_key: &'a SecretSeed,
}

pub(super) fn issue(
    policy: &ChatPolicy,
    current: &MembershipState,
    state: &ledger::State,
    original_request: RequestId,
    issuer: &LocalReceiptIssuer<'_>,
) -> Result<outbox::SignedChatReceipt> {
    if issuer.policy.scope != policy.scope || current.scope != policy.scope {
        return Err(ChatStoreError::Scope);
    }
    if codec::policy_digest(&issuer.policy) != codec::policy_digest(policy) {
        return Err(ChatStoreError::Policy);
    }
    if issuer.peer.is_empty() || issuer.peer.len() > 128 {
        return Err(ChatStoreError::Limit);
    }
    let device = current
        .devices
        .get(&issuer.receiver.device)
        .ok_or(IdentityError::UnknownDevice)?;
    if device.revoked {
        return Err(IdentityError::Revoked.into());
    }
    if device.account != issuer.receiver.account
        || device.peer.as_slice() != issuer.peer
        || device.key != issuer.device_key.public_key()
    {
        return Err(ChatStoreError::Signature);
    }
    let account = current
        .accounts
        .get(&issuer.receiver.account)
        .ok_or(IdentityError::UnknownAccount)?;
    if !account.roles.contains(Roles::PLAYER) {
        return Err(IdentityError::RolePolicy.into());
    }
    if original_request.as_bytes() == &[0; 16] {
        return Err(ChatStoreError::Malformed);
    }
    let request = state
        .requests
        .get(&original_request)
        .ok_or(ChatStoreError::Conflict)?;
    let entry = state
        .messages
        .get(&request.message)
        .ok_or(ChatStoreError::Corrupt)?;
    // A deduplicated alias is evidence of the same message, never the first original request.
    if entry.receipt.original_request != original_request {
        return Err(ChatStoreError::Conflict);
    }
    // schema::verify already authenticated the canonical signed original and every retained alias.
    // Later sender revocation does not erase it; fresh issuer PLAYER authority is checked above.
    outbox::sign_delivery(
        policy,
        issuer.receiver,
        issuer.peer,
        &entry.receipt,
        &entry.signed,
        issuer.device_key,
    )
}
