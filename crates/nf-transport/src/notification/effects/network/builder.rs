use super::NotifyOwnerCodec;
use crate::{PeerError, mux::LaneMuxConfig, notification::PROTOCOL, records::Lane};
use libp2p::{StreamProtocol, Swarm, SwarmBuilder, request_response};
use std::{
    num::{NonZeroU8, NonZeroUsize},
    time::Duration,
};
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct NotifyBehaviour {
    pub messages: request_response::Behaviour<NotifyOwnerCodec>,
    limits: libp2p::connection_limits::Behaviour,
}
pub(crate) fn build_notify_swarm(
    key: libp2p::identity::Keypair,
) -> Result<Swarm<NotifyBehaviour>, PeerError> {
    let limits = libp2p::connection_limits::ConnectionLimits::default()
        .with_max_pending_incoming(Some(1))
        .with_max_pending_outgoing(Some(1))
        .with_max_established(Some(1))
        .with_max_established_per_peer(Some(1));
    let swarm = SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            || LaneMuxConfig::new(Lane::Bulk),
        )
        .map_err(|_| PeerError::Offline)?
        .with_behaviour(move |_| NotifyBehaviour {
            messages: request_response::Behaviour::with_codec(
                NotifyOwnerCodec::default(),
                [(
                    StreamProtocol::new(PROTOCOL),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default()
                    .with_request_timeout(Duration::from_secs(5))
                    .with_max_concurrent_streams(1),
            ),
            limits: libp2p::connection_limits::Behaviour::new(limits),
        })
        .map_err(|_| PeerError::Offline)?
        .with_swarm_config(|c| {
            c.with_per_connection_event_buffer_size(16)
                .with_notify_handler_buffer_size(NonZeroUsize::new(16).expect("nonzero"))
                .with_max_negotiating_inbound_streams(4)
                .with_dial_concurrency_factor(NonZeroU8::new(1).expect("nonzero"))
                .with_idle_connection_timeout(Duration::from_secs(61))
        })
        .build();
    Ok(swarm)
}
