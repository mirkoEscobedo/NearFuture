use nf_contract::identity::{AccountId, DeviceId};
use nf_identity::private_storage::PrivateVault;
use nf_ipc::{DiscoveryRecord, LocalPrincipal, PublishedRendezvous, SessionConfig};
use std::{
    fs,
    net::{Ipv4Addr, SocketAddr},
};
#[test]
fn owner_private_descriptor_attaches_without_public_token_and_removes_only_own_record() {
    let root = std::env::temp_dir().join(format!("nf-ipc-rendezvous-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let saves = root.join("saves");
    fs::create_dir(&saves).unwrap();
    let vault = PrivateVault::create(&root.join("vault"), &saves).unwrap();
    let config = SessionConfig {
        universe: [1; 16],
        history: [2; 16],
        runtime_session: 9,
        ruleset: [3; 32],
        content_policy: [4; 32],
        limits: nf_ipc::default_limits(),
    };
    let principal = LocalPrincipal {
        account: AccountId::from_bytes([5; 16]),
        device: DeviceId::from_bytes([6; 16]),
    };
    let published = PublishedRendezvous::publish(
        &vault,
        "ipc-test",
        config,
        principal,
        SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
    )
    .unwrap();
    let attached = DiscoveryRecord::read(&vault, "ipc-test").unwrap();
    assert_eq!(attached.address().port(), 12345);
    let auth = published
        .record()
        .authenticator(nf_ipc::EndpointRole::Control)
        .unwrap();
    let client = attached.client(nf_ipc::EndpointRole::Control).unwrap();
    let challenge = auth.begin(client.hello()).unwrap();
    let proof = client.respond(challenge.challenge()).unwrap();
    let (_, accepted) = auth.finish(challenge, proof.proof()).unwrap();
    proof.finish(&accepted).unwrap();
    assert!(
        PublishedRendezvous::publish(
            &vault,
            "ipc-test",
            attached.config().clone(),
            principal,
            attached.address()
        )
        .is_err()
    );
    published.close().unwrap();
    assert!(DiscoveryRecord::read(&vault, "ipc-test").is_err());
    let original = PublishedRendezvous::publish(
        &vault,
        "ipc-test",
        attached.config().clone(),
        principal,
        attached.address(),
    )
    .unwrap();
    vault.remove_private_blob("ipc-test").unwrap();
    let mut changed = attached.config().clone();
    changed.runtime_session = 10;
    let replacement = PublishedRendezvous::publish(
        &vault,
        "ipc-test",
        changed,
        principal,
        SocketAddr::from((Ipv4Addr::LOCALHOST, 54321)),
    )
    .unwrap();
    assert!(original.close().is_err());
    let retained = DiscoveryRecord::read(&vault, "ipc-test").unwrap();
    assert_eq!(retained.config().runtime_session, 10);
    assert_eq!(retained.address().port(), 54321);
    replacement.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
