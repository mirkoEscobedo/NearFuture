use super::*;
use crate::{PeerError, receipt::*};
use nf_identity::private_storage::PrivateVault;
use std::{cell::Cell, path::PathBuf};
thread_local! { static COLLIDE: Cell<bool> = const { Cell::new(false) }; }
pub(super) fn before_create(vault: &PrivateVault, name: &str) -> Result<(), PeerError> {
    if COLLIDE.replace(false) {
        vault
            .create_private_blob(name, b"malformed owned test tail")
            .map_err(|_| PeerError::Storage)?;
    }
    Ok(())
}
struct Armed;
impl Armed {
    fn new() -> Self {
        assert!(!COLLIDE.replace(true));
        Self
    }
}
impl Drop for Armed {
    fn drop(&mut self) {
        COLLIDE.set(false);
    }
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn actual_create_new_refusal_preserves_old_prefix_and_rejects_inserted_tail() {
    let mut nonce = [0; 16];
    getrandom::fill(&mut nonce).unwrap();
    let suffix = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let scratch = Scratch(std::env::temp_dir().join(format!("nf-book-collision-{suffix}")));
    std::fs::create_dir(&scratch.0).unwrap();
    std::fs::create_dir(scratch.0.join("saves")).unwrap();
    let vault = PrivateVault::create(&scratch.0.join("vault"), &scratch.0.join("saves")).unwrap();
    let input = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/transport/vectors/receipt-v2.tsv"
    ));
    let raw = input
        .lines()
        .find_map(|line| {
            let f = line.split('\t').collect::<Vec<_>>();
            (f.first() == Some(&"book") && f.get(1) == Some(&"generation-0")).then(|| f[4])
        })
        .unwrap();
    let bytes = raw
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    let record = decode_book(&bytes).unwrap();
    let catalog = initialize_book(&vault, &[(0, record.clone())]).unwrap();
    let anchors = catalog.anchors().unwrap();
    let before = vault.read_private_blob("receipt-r0-g0").unwrap();
    let o = record.original.original();
    let status = ReceiptStatus {
        request: o.request_id,
        operation: record.original.operation(),
        account: o.account_id,
        device: o.device_id,
        binding: record.original.binding_digest(),
        current: record.minimum,
        phase: ReceiptPhase::Pending,
    };
    let _armed = Armed::new();
    let failure = append_receipt_detailed(&vault, &anchors, 0, &status, record.minimum)
        .err()
        .unwrap();
    assert_eq!(failure.peer_error(), PeerError::Storage);
    let diagnostic = failure
        .storage_failure()
        .and_then(|f| f.diagnostic())
        .expect("append create-new cause missing");
    assert_eq!(
        diagnostic.operation(),
        nf_identity::private_storage::PrivateOperation::BlobCreate
    );
    assert_eq!(
        diagnostic.stage(),
        nf_identity::private_storage::PrivateStage::CreateNew
    );
    assert!(matches!(
        diagnostic.cause(),
        nf_identity::private_storage::PrivateCause::Io {
            kind: std::io::ErrorKind::AlreadyExists,
            ..
        }
    ));
    assert_eq!(vault.read_private_blob("receipt-r0-g0").unwrap(), before);
    assert_eq!(anchors[0].generation, 0);
    assert!(recover_book(&vault, &anchors).is_err());
}
