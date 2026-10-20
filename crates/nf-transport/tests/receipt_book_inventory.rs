#[path = "receipt_effect_support/corpus.rs"]
mod corpus;
#[path = "receipt_effect_support/scratch.rs"]
mod scratch;
use nf_transport::{PeerError, receipt_effects::book::*};
#[test]
fn entire_inventory_refuses_an_unconfigured_tail_after_an_apparent_end() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    vault
        .create_private_blob("receipt-r7-g7", &corpus::row("book", "generation-7"))
        .unwrap();
    assert!(
        recover_book(&vault, &[]).is_err(),
        "present unconfigured tail was skipped"
    );
}
#[test]
fn configured_chain_gap_and_corrupt_tail_never_return_an_old_prefix() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    let root = decode_book(&corpus::row("book", "generation-0")).unwrap();
    let anchor = BookAnchor::from_record(0, &root).unwrap();
    vault
        .create_private_blob("receipt-r0-g0", &encode_book(&root).unwrap())
        .unwrap();
    assert_eq!(
        recover_book(&vault, std::slice::from_ref(&anchor))
            .unwrap()
            .head(0)
            .unwrap()
            .generation,
        0
    );
    vault
        .create_private_blob("receipt-r0-g2", &corpus::row("book", "generation-2"))
        .unwrap();
    assert!(
        recover_book(&vault, std::slice::from_ref(&anchor)).is_err(),
        "generation gap admitted"
    );
    vault.remove_private_blob("receipt-r0-g2").unwrap();
    vault
        .create_private_blob("receipt-r0-g7", b"bad tail")
        .unwrap();
    assert!(
        matches!(recover_book(&vault, &[anchor]), Err(PeerError::Limit)),
        "corrupt tail was skipped"
    );
}
