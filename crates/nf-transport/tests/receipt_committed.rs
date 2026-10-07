#[path = "receipt_support/corpus.rs"]
mod corpus;
use nf_contract::identity::EventSeq;
use nf_transport::{
    receipt::{ReceiptBody, ReceiptPhase, decode_body},
    records::PeerLimits,
};
#[test]
fn committed_receipt_is_retained_metadata_with_historical_sequence() {
    assert_eq!(corpus::matching("record", "shape").len(), 14);
    let raw = corpus::row("record", "status-committed");
    let result = decode_body(&raw, PeerLimits::default());
    assert!(
        result.is_ok(),
        "control2 must admit the independent committed receipt metadata: {result:?}"
    );
    let ReceiptBody::Status { status, .. } = result.unwrap().body else {
        panic!("receipt status")
    };
    assert_eq!(
        status.phase,
        ReceiptPhase::Committed {
            sequence: EventSeq(7)
        }
    );
    assert!(
        status.current.event > EventSeq(7),
        "historical commit may predate current source frontier"
    );
}
