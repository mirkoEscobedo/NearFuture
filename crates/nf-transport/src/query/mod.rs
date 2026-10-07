//! Single background owner of retained-status queries. No request route can mutate world or membership.
mod client;
mod server;
use crate::{
    PeerError,
    records::{RetainedPhase, UnsupportedReason},
    session::Binding,
};
pub use client::QueryClient;
use nf_identity::model::{MembershipRepository, MembershipState};
use nf_store::Store;
pub use server::QueryServer;
use std::time::{Duration, Instant};
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryResult {
    Status(RetainedPhase),
    Unsupported(UnsupportedReason),
}
pub(super) fn current(store: &mut Store, b: &Binding<'_>) -> Result<MembershipState, PeerError> {
    let context = b.context.context;
    let known = store.known_frontiers().map_err(|_| PeerError::Storage)?;
    if known.scope != context.scope || store.world().to_spec().ruleset_hash != context.ruleset {
        return Err(PeerError::Policy);
    }
    let state = store
        .load_membership(context.scope)
        .map_err(|_| PeerError::Storage)?
        .ok_or(PeerError::Unauthorized)?;
    b.current(&state)?;
    Ok(state)
}
pub(crate) fn fresh_time(created: Instant, now: Instant) -> Result<(), PeerError> {
    if now
        .checked_duration_since(created)
        .is_none_or(|d| d >= Duration::from_secs(5))
    {
        return Err(PeerError::Replay);
    }
    Ok(())
}
