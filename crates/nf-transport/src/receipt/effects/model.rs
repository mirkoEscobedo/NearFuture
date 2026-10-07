use crate::{
    PeerError,
    receipt::{ReceiptPhase, ReceiptTarget, SourceMinima},
};
use libp2p::PeerId;
use nf_contract::identity::*;
use nf_identity::{
    keys::SecretSeed,
    model::{MembershipState, PublicIdentity, Scope},
};
/// Ephemeral trusted borrow. Returning owned observations creates no later grant.
pub(crate) struct CurrentRead<'a> {
    pub(crate) membership: &'a MembershipState,
    pub(crate) local: &'a PublicIdentity,
    pub(crate) device_key: &'a SecretSeed,
    pub(crate) source: SourceMinima,
}
/// Constructed only by a freshly checked active protocol session, never wire principal claims.
pub(crate) struct RemotePrincipal {
    pub(crate) account: AccountId,
    pub(crate) device: DeviceId,
    pub(crate) scope: Scope,
    pub(crate) peer: PeerId,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptSelector {
    pub(super) account: AccountId,
    pub(super) device: DeviceId,
    pub(super) scope: Scope,
    pub(super) peer: PeerId,
    pub(super) target: ReceiptTarget,
}
impl ReceiptSelector {
    pub fn target(&self) -> ReceiptTarget {
        self.target
    }
    pub fn account(&self) -> AccountId {
        self.account
    }
    pub fn device(&self) -> DeviceId {
        self.device
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
}
#[derive(Clone, Debug)]
pub struct ProjectionRead {
    pub selector: ReceiptSelector,
    pub phase: ReceiptPhase,
    pub source: SourceMinima,
}
pub(super) fn role(
    s: &MembershipState,
    account: AccountId,
    device: DeviceId,
    peer: PeerId,
) -> Result<(), PeerError> {
    let a = s.accounts.get(&account).ok_or(PeerError::Unauthorized)?;
    let d = s.devices.get(&device).ok_or(PeerError::Unauthorized)?;
    if !a.roles.contains(nf_identity::model::Roles::PLAYER)
        || d.revoked
        || d.account != account
        || d.peer != peer.to_bytes()
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
