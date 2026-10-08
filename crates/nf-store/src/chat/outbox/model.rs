use super::receipt::SignedChatReceipt;
use crate::chat::{Author, ChatPolicy, ChatStoreError, Result, SignedMessage};
use nf_contract::identity::RequestId;
use nf_identity::model::{MembershipState, Roles};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxProfile {
    pub(super) policy: ChatPolicy,
    pub(super) sender: RetainedKey,
    pub(super) receiver: RetainedKey,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RetainedKey {
    pub actor: Author,
    pub peer: Vec<u8>,
    pub key: [u8; 32],
}
impl OutboxProfile {
    /// Trusted LOCAL supervisor selection from verified current membership, never wire enrollment.
    /// Pins remain immutable for this file's lifetime; rotation requires a separately reviewed policy.
    pub fn from_current(
        policy: &ChatPolicy,
        current: &MembershipState,
        sender: Author,
        receiver: Author,
    ) -> Result<Self> {
        nf_identity::codec::validate_state(current)?;
        if current.scope != policy.scope {
            return Err(ChatStoreError::Scope);
        }
        if sender == receiver {
            return Err(ChatStoreError::Policy);
        }
        Ok(Self {
            policy: *policy,
            sender: retained(current, sender)?,
            receiver: retained(current, receiver)?,
        })
    }
}
fn retained(current: &MembershipState, actor: Author) -> Result<RetainedKey> {
    let device = current
        .devices
        .get(&actor.device)
        .ok_or(nf_identity::model::IdentityError::UnknownDevice)?;
    let account = current
        .accounts
        .get(&actor.account)
        .ok_or(nf_identity::model::IdentityError::UnknownAccount)?;
    if device.revoked {
        return Err(nf_identity::model::IdentityError::Revoked.into());
    }
    if device.account != actor.account || device.peer.is_empty() || device.peer.len() > 128 {
        return Err(ChatStoreError::Signature);
    }
    if !account.roles.contains(Roles::PLAYER) {
        return Err(nf_identity::model::IdentityError::RolePolicy.into());
    }
    Ok(RetainedKey {
        actor,
        peer: device.peer.clone(),
        key: device.key,
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutgoingState {
    Pending,
    Delivered(Box<SignedChatReceipt>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutgoingEntry {
    pub original_request: RequestId,
    pub signed: SignedMessage,
    pub state: OutgoingState,
}
