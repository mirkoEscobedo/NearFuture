//! Proposed unit-only actual signed owner-device revocation; no authority/event fixture.
use super::{PortalEvent, PortalServer, config, support};
use crate::PeerError;
use libp2p::multiaddr::Protocol;
use nf_identity::{model::DeviceRevocation, rotation::revocation_digest};
use std::{
    net::{SocketAddr, TcpListener},
    time::{Duration, Instant},
};

#[tokio::test]
async fn actual_signed_self_device_revocation_refuses_while_original_owner_deadline_is_live() {
    let f = support::repo::RepoFixture::new();
    let config = config::server_config(&f.repo);
    let change = DeviceRevocation {
        scope: f.state.scope,
        issuer: f.server.public.account,
        device: f.server.public.device,
        frontier: f.state.revision,
    };
    let expected_revision = f.state.revision.checked_add(1).unwrap();
    let signature = f.server.account_key.sign(&revocation_digest(&change));
    let mut owner = PortalServer::new(f.repo, config, Duration::from_secs(5)).unwrap();
    let addresses = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let round = owner
                .next_round()
                .await
                .expect("actual live owner readiness");
            for event in round.events.into_iter().flatten() {
                if let PortalEvent::Ready { addresses } = event {
                    return addresses;
                }
            }
        }
    })
    .await
    .expect("all three actual listening addresses before signed self-revocation");
    assert_eq!(
        owner
            .repo
            .revoke_trusted(change, signature)
            .expect("real signed Store revocation commits"),
        expected_revision
    );
    assert!(
        owner.until.saturating_duration_since(Instant::now()) > Duration::from_secs(1),
        "the original five-second owner deadline must outlive the refusal watchdog"
    );
    let refusal = tokio::time::timeout(Duration::from_secs(1), owner.next_round())
        .await
        .expect("fresh SQL must refuse a revoked owner before its original deadline");
    assert!(matches!(refusal, Err(PeerError::Unauthorized)));
    assert!(
        Instant::now() < owner.until,
        "revocation refusal occurs before run expiry"
    );
    for _ in 0..2 {
        assert!(matches!(owner.next_round().await, Err(PeerError::Offline)));
    }
    let mut reclaimed = Vec::with_capacity(3);
    for address in addresses {
        let mut parts = address.iter();
        let (Some(Protocol::Ip4(ip)), Some(Protocol::Tcp(port)), Some(Protocol::P2p(_))) =
            (parts.next(), parts.next(), parts.next())
        else {
            panic!("actual pinned IPv4/TCP listener");
        };
        assert!(ip.is_loopback());
        assert_ne!(port, 0);
        assert!(parts.next().is_none());
        reclaimed.push(
            TcpListener::bind(SocketAddr::new(ip.into(), port))
                .expect("revocation shutdown releases every actual owned listening socket"),
        );
    }
    assert_eq!(reclaimed.len(), 3);
}
