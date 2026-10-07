#[path = "receipt_effect_support/corpus.rs"]
mod corpus;
#[path = "receipt_effect_support/scratch.rs"]
mod scratch;
use nf_transport::receipt_effects::book::*;
#[test]
fn independent_external_head_rejects_a_valid_prefix_rollback() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    for generation in 0..3 {
        vault
            .create_private_blob_detailed(
                &format!("receipt-r0-g{generation}"),
                &corpus::row("book", &format!("generation-{generation}")),
            )
            .unwrap();
    }
    let head = decode_book(&corpus::row("book", "generation-2")).unwrap();
    let anchor = BookAnchor::from_record(0, &head).unwrap();
    assert_eq!(
        recover_book(&vault, std::slice::from_ref(&anchor))
            .unwrap()
            .head(0)
            .unwrap()
            .generation,
        2
    );
    vault.remove_private_blob("receipt-r0-g2").unwrap();
    assert!(
        recover_book(&vault, std::slice::from_ref(&anchor)).is_err(),
        "valid-prefix rollback ignored external head"
    );
    assert!(
        recover_book(&vault, &[]).is_err(),
        "unconfigured existing book was adopted"
    );
}
#[test]
fn first_initialization_requires_every_reserved_name_absent_and_never_rewrites() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("private");
    let root = decode_book(&corpus::row("book", "generation-0")).unwrap();
    let initialized = initialize_book(&vault, &[(0, root.clone())]).unwrap();
    let bytes = vault.read_private_blob("receipt-r0-g0").unwrap();
    assert!(initialize_book(&vault, &[(0, root)]).is_err());
    assert_eq!(vault.read_private_blob("receipt-r0-g0").unwrap(), bytes);
    assert_eq!(initialized.occupied(), 1);
}
