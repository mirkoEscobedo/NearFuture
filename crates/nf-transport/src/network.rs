//! Low-level encrypted physical lanes only. A Noise connection or shape-admitted record grants no application authority.
use crate::{PeerError, framing::PeerCodec, mux::LaneMuxConfig, records::Lane};
use libp2p::{StreamProtocol, Swarm, SwarmBuilder, request_response};
use std::{
    num::{NonZeroU8, NonZeroUsize},
    time::Duration,
};
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct PeerBehaviour {
    pub messages: request_response::Behaviour<PeerCodec>,
    limits: libp2p::connection_limits::Behaviour,
}
pub fn build_lane_swarm(
    key: libp2p::identity::Keypair,
    lane: Lane,
) -> Result<Swarm<PeerBehaviour>, PeerError> {
    let (name, streams, connections, pending_in, pending_out) = match lane {
        Lane::Control => ("/nearfuture/peer/control/1", 4, 4, 4, 2),
        Lane::Bulk => ("/nearfuture/peer/bulk-verify/1", 1, 2, 2, 1),
    };
    let limits = libp2p::connection_limits::ConnectionLimits::default()
        .with_max_pending_incoming(Some(pending_in))
        .with_max_pending_outgoing(Some(pending_out))
        .with_max_established(Some(connections))
        .with_max_established_per_peer(Some(1));
    let swarm = SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            move || LaneMuxConfig::new(lane),
        )
        .map_err(|_| PeerError::Offline)?
        .with_behaviour(move |_| PeerBehaviour {
            messages: request_response::Behaviour::with_codec(
                PeerCodec::new(lane),
                [(
                    StreamProtocol::new(name),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default()
                    .with_request_timeout(Duration::from_secs(5))
                    .with_max_concurrent_streams(streams),
            ),
            limits: libp2p::connection_limits::Behaviour::new(limits),
        })
        .map_err(|_| PeerError::Offline)?
        .with_swarm_config(|c| {
            c.with_per_connection_event_buffer_size(16)
                .with_notify_handler_buffer_size(NonZeroUsize::new(16).expect("nonzero"))
                .with_max_negotiating_inbound_streams(4)
                .with_dial_concurrency_factor(NonZeroU8::new(1).expect("nonzero"))
                .with_idle_connection_timeout(Duration::from_secs(10))
        })
        .build();
    Ok(swarm)
}
