//! Exactly one opportunity on the private physical bulk backend.
#[cfg(test)]
use super::lifecycle_test;
use super::{PortalEvent, PortalServer};
use crate::PeerError;
use futures::Stream;
use libp2p::swarm::SwarmEvent;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
impl PortalServer {
    pub(super) fn poll_bulk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Result<Option<PortalEvent>, PeerError> {
        let event = Pin::new(self.bulk.as_mut().ok_or(PeerError::Offline)?).poll_next(cx);
        #[cfg(test)]
        lifecycle_test::observe(&event, self.bulk_listener);
        match event {
            Poll::Ready(Some(SwarmEvent::NewListenAddr { address, .. })) => {
                self.listen_observation(1, address)?;
                Ok(None)
            }
            Poll::Ready(Some(SwarmEvent::Behaviour(_))) => Err(PeerError::Unsupported),
            Poll::Ready(Some(
                SwarmEvent::ListenerError { .. }
                | SwarmEvent::ExpiredListenAddr { .. }
                | SwarmEvent::ListenerClosed { .. },
            ))
            | Poll::Ready(None) => Err(PeerError::Offline),
            Poll::Ready(Some(_)) => {
                cx.waker().wake_by_ref();
                Ok(None)
            }
            Poll::Pending => Ok(None),
        }
    }
}
