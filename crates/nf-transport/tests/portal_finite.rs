mod portal_owner_support;
#[path = "notification_effect_support/mod.rs"]
mod support;
use libp2p::multiaddr::Protocol;
use nf_transport::{
    PeerError,
    portal::{PortalEvent, PortalServer},
};
use std::{
    net::{SocketAddr, TcpListener},
    time::{Duration, Instant},
};

#[tokio::test]
async fn minimum_finite_run_ends_once_refuses_next_round_and_releases_all_three_sockets() {
    let f = support::repo::RepoFixture::new();
    let config = portal_owner_support::server_config(&f.repo);
    let original = Instant::now();
    let mut owner = PortalServer::new(f.repo, config, Duration::from_millis(500)).unwrap();
    let addresses = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let round = owner.next_round().await.expect("live owner readiness");
            for event in round.events.into_iter().flatten() {
                match event {
                    PortalEvent::Ready { addresses } => return addresses,
                    PortalEvent::Ended => {
                        panic!("run ended before its actual three listeners were ready")
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("bounded readiness watchdog");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let round = owner.next_round().await.expect("finite owner output");
            if round
                .events
                .iter()
                .flatten()
                .any(|e| matches!(e, PortalEvent::Ended))
            {
                assert_eq!(
                    round.events.iter().flatten().count(),
                    1,
                    "end admits no concurrent lane output"
                );
                return;
            }
        }
    })
    .await
    .expect("original 500 ms run must end without renewal");
    assert!(original.elapsed() >= Duration::from_millis(500));
    for _ in 0..2 {
        assert!(matches!(owner.next_round().await, Err(PeerError::Offline)));
    }
    let mut reclaimed = Vec::with_capacity(3);
    for address in addresses {
        let mut parts = address.iter();
        let (Some(Protocol::Ip4(ip)), Some(Protocol::Tcp(port)), Some(Protocol::P2p(_))) =
            (parts.next(), parts.next(), parts.next())
        else {
            panic!("actual pinned IPv4/TCP listener")
        };
        assert!(parts.next().is_none());
        assert!(ip.is_loopback() && port != 0);
        reclaimed.push(
            TcpListener::bind(SocketAddr::new(ip.into(), port))
                .expect("finite shutdown released the actual listening socket"),
        );
    }
    assert_eq!(reclaimed.len(), 3);
}
