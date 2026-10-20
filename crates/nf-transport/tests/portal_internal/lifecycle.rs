//! Shared unit-only owner lifecycle fixture and passive actual-event observation.
#[path = "../portal_owner_support/mod.rs"]
mod config;
use super::{PortalEvent, PortalServer};
use crate::notification_effects::owner_test_support as support;
#[path = "bulk_listener.rs"]
mod bulk_listener;
#[path = "revocation.rs"]
mod revocation;
#[path = "sql_delay.rs"]
mod sql_delay;
thread_local! { static ACTUAL_LOSS: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) }; }
pub(super) fn observe<E>(
    event: &std::task::Poll<Option<libp2p::swarm::SwarmEvent<E>>>,
    owned: libp2p::core::transport::ListenerId,
) {
    use libp2p::swarm::SwarmEvent;
    use std::task::Poll;
    let observation = match event {
        Poll::Ready(Some(SwarmEvent::ExpiredListenAddr { listener_id, .. }))
            if *listener_id == owned =>
        {
            Some("ExpiredListenAddr")
        }
        Poll::Ready(Some(SwarmEvent::ListenerClosed { listener_id, .. }))
            if *listener_id == owned =>
        {
            Some("ListenerClosed")
        }
        _ => None,
    };
    if let Some(kind) = observation {
        ACTUAL_LOSS.set(Some(kind));
    }
}
fn observed() -> Option<&'static str> {
    ACTUAL_LOSS.get()
}

fn clear_observed() {
    ACTUAL_LOSS.set(None);
}
#[path = "watch_status.rs"]
mod watch_status;
