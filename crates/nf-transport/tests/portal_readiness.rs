#[path = "notification_effect_support/mod.rs"]
mod support;
use nf_transport::{
    portal::{PortalEvent, PortalServer},
    portal_config::{PortalConfig, PortalMode},
};
use std::time::Duration;
#[tokio::test]
async fn actual_three_listener_readiness_uses_one_real_repository() {
    let f = support::repo::RepoFixture::new();
    let c = f.repo.config();
    let config = PortalConfig {
        mode: PortalMode::Serve,
        local: nf_store::KnownFrontiers {
            scope: c.scope,
            event_sequence: nf_contract::identity::EventSeq(0),
            store_revision: 1,
            membership_revision: Some(1),
        },
        ruleset: c.ruleset,
        content: c.content,
        local_account: c.local_account,
        local_device: c.local_device,
        server: c.server_pin,
        addresses: Some(std::array::from_fn(|_| {
            "/ip4/127.0.0.1/tcp/0".parse().unwrap()
        })),
        originals: Vec::new(),
    };
    let peer = f.server_peer;
    let mut owner = PortalServer::new(f.repo, config, Duration::from_secs(5))
        .expect("valid actual existing Repo/listeners");
    let ready = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let round = owner.next_round().await.expect("actual readiness");
            for event in round.events.into_iter().flatten() {
                if let PortalEvent::Ready { addresses } = event {
                    return addresses;
                }
            }
        }
    })
    .await
    .unwrap();
    assert_ne!(ready[0], ready[1]);
    assert_ne!(ready[1], ready[2]);
    assert_ne!(ready[0], ready[2]);
    for address in ready {
        assert_eq!(
            address.iter().last(),
            Some(libp2p::multiaddr::Protocol::P2p(peer))
        );
    }
    owner.shutdown();
}
