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
    model::{MembershipRepository, MembershipState},
    private_storage::LocalIdentity,
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
fn local_current(s: &MembershipState, local: &LocalIdentity, owner: bool) -> Result<(), PeerError> {
    nf_identity::codec::validate_state(s).map_err(|_| PeerError::Unauthorized)?;
    let a = s
        .accounts
        .get(&local.public.account)
        .ok_or(PeerError::Unauthorized)?;
    let d = s
        .devices
        .get(&local.public.device)
        .ok_or(PeerError::Unauthorized)?;
    if owner && s.owner != local.public.account
        || d.revoked
        || d.account != local.public.account
        || d.peer != local.public.peer
        || d.key != local.public.device_key
        || a.key != local.public.account_key
        || !a.roles.contains(nf_identity::model::Roles::PLAYER)
    {
        return Err(PeerError::Unauthorized);
    }
    Ok(())
}
