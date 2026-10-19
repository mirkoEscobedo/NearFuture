mod client_pair;
pub use client_pair::query_and_verify_bulk;
mod bulk_client;
pub use bulk_client::verify_bulk;
mod client;
mod connection;
mod handler;
mod server;
use crate::{PeerError, session::SessionPolicy};
pub use client::request_status;
use nf_identity::{
    model::{MembershipRepository, MembershipState, PublicIdentity, Roles},
    private_storage::{LocalIdentity, PrivateVault},
};
use nf_store::Store;
pub use server::{PeerServer, PeerServerEvent};
fn membership(store: &mut Store, policy: SessionPolicy) -> Result<MembershipState, PeerError> {
    let known = store.known_frontiers().map_err(|_| PeerError::Storage)?;
    if known.scope != policy.scope || store.world().to_spec().ruleset_hash != policy.ruleset {
        return Err(PeerError::Policy);
    }
    let s = store
        .load_membership(policy.scope)
        .map_err(|_| PeerError::Storage)?
        .ok_or(PeerError::Unauthorized)?;
    if s.scope != policy.scope || s.revision < policy.minimum_membership {
        return Err(PeerError::Unauthorized);
    }
    Ok(s)
}
/// Select only from checked durable membership and the locally loaded Noise peer.
/// Ambiguous peer bindings grant no choice of principal or signing key.
pub(crate) fn current_public(
    s: &MembershipState,
    peer: libp2p::PeerId,
    owner: bool,
) -> Result<PublicIdentity, PeerError> {
    nf_identity::codec::validate_state(s).map_err(|_| PeerError::Unauthorized)?;
    let peer = peer.to_bytes();
    let mut matches = s.devices.iter().filter(|(_, d)| d.peer == peer);
    let (id, d) = matches.next().ok_or(PeerError::Unauthorized)?;
    if matches.next().is_some() {
        return Err(PeerError::Unauthorized);
    }
    let a = s.accounts.get(&d.account).ok_or(PeerError::Unauthorized)?;
    if owner && s.owner != d.account || d.revoked || !a.roles.contains(Roles::PLAYER) {
        return Err(PeerError::Unauthorized);
    }
    Ok(PublicIdentity {
        account: d.account,
        account_key: a.key,
        device: *id,
        device_key: d.key,
        peer,
    })
}
fn current_local(
    s: &MembershipState,
    vault: &PrivateVault,
    peer: libp2p::PeerId,
    owner: bool,
) -> Result<LocalIdentity, PeerError> {
    let expected = current_public(s, peer, owner)?;
    // The loader also binds the selected stable IDs to the local vault's genesis.
    vault
        .load_membership_keys(&expected)
        .map_err(|_| PeerError::Storage)
}
