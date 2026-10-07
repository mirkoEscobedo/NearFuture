use super::*;
use nf_identity::private_storage::{PrivateCause, PrivateOperation, PrivateStage};
#[test]
fn full_repository_initialization_retains_underlying_inventory_cause_once() {
    let mut f = fixture::Fixture::new();
    f.client.anchors.clear();
    let before = f.client.vault.read_private_blob("receipt-r0-g0").unwrap();
    std::fs::create_dir(f.scratch.root.join("client-private/blob-receipt-r7-g7")).unwrap();
    let minimum = nf_transport::receipt::SourceMinima {
        event: EventSeq(0),
        store_revision: 0,
        membership_revision: f.state.revision,
    };
    let Err(failure) = f
        .client
        .initialize_originals_for_test(&[(0, f.original.clone(), minimum)])
    else {
        panic!("invalid inventory admitted");
    };
    assert_eq!(failure.peer_error(), nf_transport::PeerError::Storage);
    let diagnostic = failure
        .storage_failure()
        .and_then(|failure| failure.diagnostic())
        .expect("repository initialization storage cause missing");
    assert_eq!(diagnostic.operation(), PrivateOperation::BlobScan);
    assert_eq!(diagnostic.stage(), PrivateStage::EntryType);
    assert_eq!(diagnostic.cause(), PrivateCause::Refused);
    assert!(f.client.book_anchors().is_empty());
    assert!(
        f.client.vault.read_private_blob("receipt-r0-g0").unwrap() == before,
        "original book changed"
    );
}
