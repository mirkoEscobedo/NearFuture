#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/rotated_community.rs"]
mod rotated_community;
#[path = "support/scratch.rs"]
mod scratch;

use nf_contract::identity::RequestId;
use nf_transport::{
    auth::ServerPin,
    bulk::BulkResult,
    node::{PeerServer, PeerServerEvent, query_and_verify_bulk, request_status, verify_bulk},
    query::QueryResult,
    records::{PeerLimits, RetainedPhase},
    session::SessionPolicy,
};
use rotated_community::{Rotation, reopen_rotated};
use sha2::{Digest, Sha256};

static FIXTURES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

enum RequestMode {
    Status,
    Bulk,
    Paired,
}

async fn request_after_reopen(kind: Rotation, mode: RequestMode) {
    let _serial = FIXTURES.lock().await;
    let tmp = scratch::Scratch::new();
    let c = reopen_rotated(&tmp.0, kind);
    assert_ne!(c.server.account, c.client.account);
    let policy = SessionPolicy {
        scope: c.state.scope,
        ruleset: [4; 32],
        content: [5; 32],
        limits: PeerLimits::default(),
        minimum_membership: c.state.revision,
    };
    let pin = ServerPin {
        peer: libp2p::PeerId::from_bytes(&c.server.peer).unwrap(),
        account: c.server.account,
        device: c.server.device,
        minimum_membership: c.state.revision,
    };
    let mut server = PeerServer::new(c.server_store, &c.server_vault, policy)
        .expect("reopened owner must select current signing keys");
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        let (control, bulk) = loop {
            if let PeerServerEvent::Ready { control, bulk, peer } = server.next().await.unwrap() {
                assert_eq!(peer, pin.peer);
                assert_ne!(control, bulk);
                break (control, bulk);
            }
        };
        let bytes = vec![7; 65_536];
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let request = async {
            match mode {
                RequestMode::Status => {
                    let result = request_status(c.client_store, &c.client_vault, policy, pin,
                        control, RequestId::from_bytes([9; 16])).await.unwrap();
                    assert_eq!(result, QueryResult::Status(RetainedPhase::UnknownRequest));
                }
                RequestMode::Bulk => {
                    let result = verify_bulk(c.client_store, &c.client_vault, policy, pin,
                        bulk, &bytes).await.unwrap();
                    assert!(matches!(result, BulkResult::Verified { total: 65_536, digest: d, .. } if d == digest));
                }
                RequestMode::Paired => {
                    let (query, result) = query_and_verify_bulk(c.client_store, &c.client_vault,
                        policy, pin, control, bulk, RequestId::from_bytes([9; 16]), &bytes).await.unwrap();
                    assert_eq!(query, QueryResult::Status(RetainedPhase::UnknownRequest));
                    assert!(matches!(result, BulkResult::Verified { total: 65_536, digest: d, .. } if d == digest));
                }
            }
        };
        tokio::pin!(request);
        loop {
            tokio::select! {
                () = &mut request => { break; },
                event = server.next() => { event.unwrap(); }
            }
        }
    })
    .await
    .expect("rotated foreground request deadline");
}

#[tokio::test(flavor = "current_thread")]
async fn reopened_account_rotation_authenticates_status_over_noise() {
    request_after_reopen(Rotation::Account, RequestMode::Status).await;
}

#[tokio::test(flavor = "current_thread")]
async fn reopened_device_rotation_authenticates_status_over_noise() {
    request_after_reopen(Rotation::Device, RequestMode::Status).await;
}

#[tokio::test(flavor = "current_thread")]
async fn reopened_account_rotation_authenticates_bulk_over_noise() {
    request_after_reopen(Rotation::Account, RequestMode::Bulk).await;
}

#[tokio::test(flavor = "current_thread")]
async fn reopened_device_rotation_authenticates_bulk_over_noise() {
    request_after_reopen(Rotation::Device, RequestMode::Bulk).await;
}

#[tokio::test(flavor = "current_thread")]
async fn reopened_account_rotation_authenticates_paired_lanes_over_noise() {
    request_after_reopen(Rotation::Account, RequestMode::Paired).await;
}

#[tokio::test(flavor = "current_thread")]
async fn reopened_device_rotation_authenticates_paired_lanes_over_noise() {
    request_after_reopen(Rotation::Device, RequestMode::Paired).await;
}
