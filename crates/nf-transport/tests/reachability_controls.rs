use libp2p::{
    Multiaddr, PeerId,
    core::transport::TransportError,
    swarm::{ConnectionDenied, DialError, dial_opts::PeerCondition},
};
use nf_transport::reachability::{
    DialFailure, EndpointRefusal, classify_dial_error, validate_explicit_bootstrap,
};
use std::{io, net::Ipv4Addr};

fn pin(byte: u8) -> PeerId {
    let mut bytes = [byte; 34];
    bytes[0] = 0x12;
    bytes[1] = 0x20;
    PeerId::from_bytes(&bytes).unwrap()
}
fn endpoint(ip: Ipv4Addr, port: u16, peer: PeerId) -> Multiaddr {
    format!("/ip4/{ip}/tcp/{port}/p2p/{peer}").parse().unwrap()
}

#[test]
fn explicit_loopback_peer_is_the_only_supported_route() {
    let peer = pin(7);
    let address = endpoint(Ipv4Addr::LOCALHOST, 1234, peer);
    assert_eq!(validate_explicit_bootstrap(peer, Some(&address)), Ok(()));
}

#[test]
fn missing_configuration_has_an_explicit_refusal() {
    assert_eq!(
        validate_explicit_bootstrap(pin(7), None),
        Err(EndpointRefusal::NoConfiguredPeer)
    );
}

#[test]
fn relay_is_refused_without_claiming_reservation_support() {
    let peer = pin(7);
    let address: Multiaddr = format!("/ip4/127.0.0.1/tcp/1234/p2p/{peer}/p2p-circuit")
        .parse()
        .unwrap();
    assert_eq!(
        validate_explicit_bootstrap(peer, Some(&address)),
        Err(EndpointRefusal::RelayUnsupported)
    );
}

#[test]
fn remote_ipv4_is_refused_before_dial() {
    let peer = pin(7);
    let address = endpoint(Ipv4Addr::new(192, 0, 2, 1), 1234, peer);
    assert_eq!(
        validate_explicit_bootstrap(peer, Some(&address)),
        Err(EndpointRefusal::RemoteRouteUnsupported)
    );
}

#[test]
fn zero_port_is_refused() {
    let peer = pin(7);
    let address = endpoint(Ipv4Addr::LOCALHOST, 0, peer);
    assert_eq!(
        validate_explicit_bootstrap(peer, Some(&address)),
        Err(EndpointRefusal::ZeroPort)
    );
}

#[test]
fn peer_pin_mismatch_is_refused() {
    let address = endpoint(Ipv4Addr::LOCALHOST, 1234, pin(8));
    assert_eq!(
        validate_explicit_bootstrap(pin(7), Some(&address)),
        Err(EndpointRefusal::WrongPeerPin)
    );
}

#[test]
fn unsupported_address_shape_is_refused() {
    let peer = pin(7);
    let address: Multiaddr = format!("/ip6/::1/tcp/1234/p2p/{peer}").parse().unwrap();
    assert_eq!(
        validate_explicit_bootstrap(peer, Some(&address)),
        Err(EndpointRefusal::AddressShapeUnsupported)
    );
}

#[test]
fn oversized_address_is_refused_before_protocol_scan() {
    let address: Multiaddr = ("/ip4/127.0.0.1/tcp/1234".repeat(64)).parse().unwrap();
    assert_eq!(
        validate_explicit_bootstrap(pin(7), Some(&address)),
        Err(EndpointRefusal::AddressTooLong)
    );
}

#[test]
fn no_addresses_and_aborted_dials_have_literal_codes() {
    assert_eq!(
        classify_dial_error(&DialError::NoAddresses),
        DialFailure::NoAddresses
    );
    assert_eq!(
        classify_dial_error(&DialError::Aborted),
        DialFailure::Aborted
    );
}

#[test]
fn wrong_peer_and_local_peer_dials_have_literal_codes() {
    let address = endpoint(Ipv4Addr::LOCALHOST, 1234, pin(7));
    assert_eq!(
        classify_dial_error(&DialError::WrongPeerId {
            obtained: pin(8),
            address: address.clone()
        }),
        DialFailure::WrongPeer
    );
    assert_eq!(
        classify_dial_error(&DialError::LocalPeerId { address }),
        DialFailure::LocalPeer
    );
}

#[test]
fn raw_transport_error_and_endpoint_do_not_enter_the_code() {
    let error = DialError::Transport(vec![(
        endpoint(Ipv4Addr::new(192, 0, 2, 33), 4321, pin(7)),
        TransportError::Other(io::Error::other("private/path/secret-address")),
    )]);
    let code = classify_dial_error(&error);
    assert_eq!(code, DialFailure::TransportNegotiation);
    assert_eq!(format!("{code:?}"), "TransportNegotiation");
}

#[test]
fn suppressed_and_behaviour_denied_dials_have_literal_codes() {
    assert_eq!(
        classify_dial_error(&DialError::DialPeerConditionFalse(
            PeerCondition::NotDialing
        )),
        DialFailure::DialSuppressed
    );
    let error = DialError::Denied {
        cause: ConnectionDenied::new(io::Error::other("private/denial/path")),
    };
    let code = classify_dial_error(&error);
    assert_eq!(code, DialFailure::DeniedByBehaviour);
    assert_eq!(format!("{code:?}"), "DeniedByBehaviour");
}
