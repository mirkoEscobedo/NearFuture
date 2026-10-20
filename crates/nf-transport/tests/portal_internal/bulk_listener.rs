//! Real listener removal; no fabricated SwarmEvent or application authority setter.
use super::{PortalEvent, PortalServer, clear_observed, config, observed, support};
use crate::PeerError;
use libp2p::multiaddr::Protocol;
use std::{
    net::{SocketAddr, TcpListener},
    time::Duration,
};

#[tokio::test]
async fn actual_owned_bulk_listener_loss_refuses_before_original_run_expiry() {
    let f = support::repo::RepoFixture::new();
    let config = config::server_config(&f.repo);
    let mut owner = PortalServer::new(f.repo, config, Duration::from_secs(5)).unwrap();
    let addresses = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let round = owner.next_round().await.expect("actual owner readiness");
            for event in round.events.into_iter().flatten() {
                if let PortalEvent::Ready { addresses } = event {
                    return addresses;
                }
            }
        }
    })
    .await
    .expect("actual three listeners before listener loss");
    clear_observed();
    assert!(
        owner
            .bulk
            .as_mut()
            .unwrap()
            .remove_listener(owner.bulk_listener),
        "remove the actual listener created by this private owner backend"
    );
    let refusal = tokio::time::timeout(Duration::from_secs(1), owner.next_round()).await;
    assert!(
        observed().is_some(),
        "real owned listener loss must reach the actual backend poll"
    );
    let refusal = refusal.unwrap_or_else(|_| panic!(
        "owned bulk listener loss must refuse before the 5 s run ends; actual observed event: {:?}", observed()));
    assert!(matches!(refusal, Err(PeerError::Offline)));
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
        reclaimed.push(
            TcpListener::bind(SocketAddr::new(ip.into(), port))
                .expect("listener loss shutdown releases every owned listening socket"),
        );
    }
    assert_eq!(reclaimed.len(), 3);
}
