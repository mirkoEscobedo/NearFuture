//! First protected Pending persistence must fit the existing public Status budget.
#[path = "receipt_effect_support/corpus.rs"]
mod corpus;
#[path = "receipt_effect_support/scratch.rs"]
mod scratch;
use nf_transport::{receipt::*, receipt_effects::book::*};
use std::time::{Duration, Instant};

#[test]
fn first_pending_receipt_persistence_fits_public_status_budget() {
    let scratch = scratch::Scratch::new();
    // Existing real vault setup must complete before the measured append starts.
    let vault = scratch.vault("book");
    let original = decode_book(&corpus::row("book", "generation-0"))
        .expect("SETUP independent generation-zero corpus");
    assert_eq!(original.generation, 0);
    assert_eq!(original.phase, BookPhase::Unobserved);
    let initialized = initialize_book(&vault, &[(0, original.clone())])
        .expect("SETUP actual protected book initialization");
    let anchors = initialized.anchors().expect("SETUP actual external anchor");
    let genesis_bytes = vault
        .read_private_blob("receipt-r0-g0")
        .expect("SETUP actual genesis bytes");
    let request = original.original.original();
    let status = ReceiptStatus {
        request: request.request_id,
        operation: original.original.operation(),
        account: request.account_id,
        device: request.device_id,
        binding: original.original.binding_digest(),
        current: original.minimum,
        phase: ReceiptPhase::Pending,
    };
    let expected = BookRecord {
        generation: 1,
        previous: original.digest().unwrap(),
        phase: BookPhase::Receipt(ReceiptPhase::Pending),
        ..original.clone()
    };
    expected.follows(&original).unwrap();

    let started = Instant::now();
    let appended = append_receipt(&vault, &anchors, 0, &status, original.minimum);
    let elapsed = started.elapsed();
    let appended =
        appended.expect("protected Pending append must succeed before latency classification");
    let head = appended.head(0).unwrap();
    assert_eq!(head.generation, 1);
    assert_eq!(head.phase, BookPhase::Receipt(ReceiptPhase::Pending));
    assert!(head.same_original(&original));
    assert_eq!(head.original.binding_digest(), status.binding);
    assert_eq!(head.minimum, status.current);
    assert_eq!(head.digest().unwrap(), expected.digest().unwrap());
    let recovered = recover_book(&vault, &appended.anchors().unwrap()).unwrap();
    assert_eq!(
        recovered.head(0).unwrap().digest().unwrap(),
        head.digest().unwrap()
    );
    assert_eq!(
        vault.read_private_blob("receipt-r0-g0").unwrap(),
        genesis_bytes
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "successful protected first Pending append alone exceeded the existing public Status budget: {elapsed:?}"
    );
}
