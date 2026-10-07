use super::*;
#[path = "../../../../tests/receipt_effect_support/corpus.rs"]
mod corpus;
use crate::PeerError;
use crate::receipt_effects::tests::repository::scratch;
use nf_identity::private_storage::{PrivateCause, PrivateOperation, PrivateStage};
#[test]
fn detailed_recovery_preserves_scan_metadata_cause() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    std::fs::remove_dir(scratch.root.join("private")).unwrap();
    let failure = recover_book_detailed(&vault, &[]).err().unwrap();
    assert_eq!(failure.peer_error(), PeerError::Storage);
    let diagnostic = failure
        .storage_failure()
        .and_then(|f| f.diagnostic())
        .expect("underlying storage cause missing");
    assert_eq!(diagnostic.operation(), PrivateOperation::BlobScan);
    assert_eq!(diagnostic.stage(), PrivateStage::RootMetadata);
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }
    ));
}

#[test]
fn detailed_initialization_preserves_underlying_recovery_cause() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    let record = decode_book(&corpus::row("book", "generation-0")).unwrap();
    std::fs::remove_dir(scratch.root.join("private")).unwrap();
    let failure = initialize_book_detailed(&vault, &[(0, record)])
        .err()
        .unwrap();
    assert_eq!(failure.peer_error(), PeerError::Storage);
    let diagnostic = failure
        .storage_failure()
        .and_then(|f| f.diagnostic())
        .expect("initialization storage cause missing");
    assert_eq!(diagnostic.operation(), PrivateOperation::BlobScan);
    assert_eq!(diagnostic.stage(), PrivateStage::RootMetadata);
    assert!(matches!(
        diagnostic.cause(),
        PrivateCause::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }
    ));
}
#[test]
fn semantic_callback_error_has_precedence_and_no_storage_diagnostic() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    let bytes = corpus::row("book", "generation-0");
    let record = decode_book(&bytes).unwrap();
    let anchor = BookAnchor::from_record(0, &record).unwrap();
    vault.create_private_blob("receipt-r0-g0", &bytes).unwrap();
    vault
        .create_private_blob("receipt-r0-g7", b"bad tail")
        .unwrap();
    let Err(failure) = recover_book_detailed(&vault, &[anchor]) else {
        panic!("corrupt tail admitted");
    };
    assert_eq!(failure.peer_error(), PeerError::Limit);
    assert_eq!(failure.storage_failure(), None);
}
