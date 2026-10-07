#[path = "receipt_effect_support/corpus.rs"]
mod corpus;
use nf_contract::identity::OperationId;
use nf_transport::{PeerError, receipt::*, receipt_effects::book::*};
#[test]
fn canonical_new_generation_cannot_change_original_pin_payload_or_terminal_phase() {
    let old = decode_book(&corpus::row("book", "generation-2")).unwrap();
    let next = decode_book(&corpus::row("book", "generation-3")).unwrap();
    next.follows(&old).unwrap();
    for case in 0..6 {
        let mut changed = next.clone();
        let mut original = *changed.original.original();
        let mut pin = changed.original.source();
        let mut operation = changed.original.operation();
        match case {
            0 => original.payload_digest[0] ^= 1,
            1 => operation = OperationId::from_bytes([77; 16]),
            2 => {
                pin.peer = libp2p::identity::Keypair::generate_ed25519()
                    .public()
                    .to_peer_id()
            }
            3 => changed.ruleset[0] ^= 1,
            4 => changed.content[0] ^= 1,
            _ => changed.phase = BookPhase::Receipt(ReceiptPhase::Unknown),
        }
        changed.original = OriginalReceipt::new(operation, original, pin).unwrap();
        // Each independently canonical record remains structurally valid; the chain rejects substitution.
        decode_book(&encode_book(&changed).unwrap()).unwrap();
        assert!(matches!(changed.follows(&old), Err(PeerError::Replay)));
    }
}
