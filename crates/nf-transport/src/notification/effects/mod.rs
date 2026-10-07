//! Connection-owned notification transitions. Integration must supply a fresh sole-repository read at each protected stage.
mod binding;
mod handshake;
mod policy;
pub(crate) use binding::{NotifyEndpoint, NotifySession};
pub(crate) use handshake::{NotifyClientHandshake, NotifyServerHandshake};
pub use policy::NotifyPolicy;
mod broker;
mod owner;
pub(crate) use broker::NotifyBroker;
mod quota;
pub use quota::NotifyQueueStats;
pub(crate) use quota::{NotifyPermit, NotifyQuota};

mod client;
pub(crate) use client::NotifySubscriber;

mod flow;
pub(crate) use flow::NotifyFlow;

mod network;
pub(crate) use network::build_notify_swarm;
pub use network::{NotifyBehaviour, NotifyCodec, NotifyOwnerCodec, NotifyRequest};

mod emission;
pub(crate) use emission::NotifyEmission;

mod delivery;

mod lane;
pub use lane::{NotifyLane, NotifyLaneEvent};

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use tests::support as owner_test_support;

fn elapsed_cut(
    created: std::time::Instant,
    duration: std::time::Duration,
) -> Result<(), crate::PeerError> {
    if created.elapsed() >= duration {
        Err(crate::PeerError::Replay)
    } else {
        Ok(())
    }
}

fn until_cut(end: std::time::Instant) -> Result<(), crate::PeerError> {
    if std::time::Instant::now() >= end {
        Err(crate::PeerError::Replay)
    } else {
        Ok(())
    }
}
