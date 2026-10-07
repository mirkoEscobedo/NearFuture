#[path = "receipt_effect_support/corpus.rs"]
mod corpus;
#[path = "receipt_effect_support/scratch.rs"]
mod scratch;
use nf_contract::identity::{OperationId, RequestId};
use nf_transport::{PeerError, receipt::*, receipt_effects::book::*};
#[test]
fn full_closed_inventory_recovers_and_gen7_refuses_raised_minima_without_new_bytes() {
    let scratch = scratch::Scratch::new();
    let vault = scratch.vault("book");
    let first = decode_book(&corpus::row("book", "generation-0")).unwrap();
    let mut anchors = Vec::new();
    for slot in 0u8..8 {
        let mut original = *first.original.original();
        original.request_id = RequestId::from_bytes([slot + 1; 16]);
        let original = OriginalReceipt::new(
            OperationId::from_bytes([slot + 10; 16]),
            original,
            first.original.source(),
        )
        .unwrap();
        let mut previous = [0; 32];
        for generation in 0u8..8 {
            let mut record =
                decode_book(&corpus::row("book", &format!("generation-{generation}"))).unwrap();
            record.original = original.clone();
            record.previous = previous;
            let bytes = encode_book(&record).unwrap();
            vault
                .create_private_blob_detailed(&format!("receipt-r{slot}-g{generation}"), &bytes)
                .unwrap();
            previous = record.digest().unwrap();
            if generation == 7 {
                anchors.push(BookAnchor::from_record(slot, &record).unwrap());
            }
        }
    }
    let catalog = recover_book(&vault, &anchors).unwrap();
    assert_eq!(catalog.occupied(), 8);
    let old = catalog.head(0).unwrap();
    let BookPhase::Receipt(phase) = old.phase else {
        panic!("receipt");
    };
    let o = old.original.original();
    let mut status = ReceiptStatus {
        request: o.request_id,
        operation: old.original.operation(),
        account: o.account_id,
        device: o.device_id,
        binding: old.original.binding_digest(),
        current: old.minimum,
        phase,
    };
    assert_eq!(
        encode_book(
            append_receipt(&vault, &anchors, 0, &status, old.minimum)
                .unwrap()
                .head(0)
                .unwrap()
        )
        .unwrap(),
        encode_book(old).unwrap()
    );
    let old_bytes = vault.read_private_blob("receipt-r0-g7").unwrap();
    status.current.store_revision += 1;
    assert!(matches!(
        append_receipt(&vault, &anchors, 0, &status, old.minimum),
        Err(PeerError::Backpressure)
    ));
    assert_eq!(vault.read_private_blob("receipt-r0-g7").unwrap(), old_bytes);
    assert_eq!(
        encode_book(recover_book(&vault, &anchors).unwrap().head(0).unwrap()).unwrap(),
        encode_book(old).unwrap()
    );
}
