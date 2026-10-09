//! Bounded diagnostics for the existing explicitly pinned loopback control route.
//! Observations describe this completed attempt, not current connectivity or authority.
use crate::{PeerError, query::QueryResult};
use libp2p::{Multiaddr, PeerId, multiaddr::Protocol, swarm::DialError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointRefusal {
    NoConfiguredPeer,
    AddressTooLong,
    RelayUnsupported,
    AddressShapeUnsupported,
    RemoteRouteUnsupported,
    ZeroPort,
    WrongPeerPin,
}

/// Relay, discovery, and remote routes are not installed in this transport.
/// A successful check authorizes no membership or application request.
pub fn validate_explicit_bootstrap(
    pin: PeerId,
    address: Option<&Multiaddr>,
) -> Result<(), EndpointRefusal> {
    let address = address.ok_or(EndpointRefusal::NoConfiguredPeer)?;
    if address.len() > 256 {
        return Err(EndpointRefusal::AddressTooLong);
    }
    if address
        .iter()
        .any(|part| matches!(part, Protocol::P2pCircuit))
    {
        return Err(EndpointRefusal::RelayUnsupported);
    }
    let mut parts = address.iter();
    let (Some(Protocol::Ip4(ip)), Some(Protocol::Tcp(port)), Some(Protocol::P2p(peer)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(EndpointRefusal::AddressShapeUnsupported);
    };
    if !ip.is_loopback() {
        return Err(EndpointRefusal::RemoteRouteUnsupported);
    }
    if port == 0 {
        return Err(EndpointRefusal::ZeroPort);
    }
    if peer != pin {
        return Err(EndpointRefusal::WrongPeerPin);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialFailure {
    LocalPeer,
    NoAddresses,
    DialSuppressed,
    Aborted,
    WrongPeer,
    DeniedByBehaviour,
    TransportNegotiation,
}

/// Deliberately excludes addresses, peer IDs, and nested transport error text.
pub fn classify_dial_error(error: &DialError) -> DialFailure {
    match error {
        DialError::LocalPeerId { .. } => DialFailure::LocalPeer,
        DialError::NoAddresses => DialFailure::NoAddresses,
        DialError::DialPeerConditionFalse(_) => DialFailure::DialSuppressed,
        DialError::Aborted => DialFailure::Aborted,
        DialError::WrongPeerId { .. } => DialFailure::WrongPeer,
        DialError::Denied { .. } => DialFailure::DeniedByBehaviour,
        DialError::Transport(_) => DialFailure::TransportNegotiation,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Observation {
    #[default]
    NotDialed,
    DialAttempted,
    DirectConnectionObserved,
    ReplyValidated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Endpoint(EndpointRefusal),
    Dial(DialFailure),
    ConnectionClosed,
    RequestResponse,
    Deadline,
    Operation(PeerError),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Diagnostic {
    pub(crate) observation: Observation,
    pub(crate) failure: Option<Failure>,
}

impl Diagnostic {
    pub fn observation(&self) -> Observation {
        self.observation
    }
    pub fn failure(&self) -> Option<Failure> {
        self.failure
    }
}

/// The query result retains its original authorization and persistence semantics.
pub struct StatusReport {
    pub result: Result<QueryResult, PeerError>,
    pub diagnostic: Diagnostic,
}
