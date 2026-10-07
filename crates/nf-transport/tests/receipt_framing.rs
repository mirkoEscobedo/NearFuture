use futures::{executor::block_on, io::Cursor};
use libp2p::{StreamProtocol, request_response::Codec};
use nf_transport::receipt::{RECEIPT_PROTOCOL, ReceiptCodec};
#[test]
fn impossible_receipt_frame_declaration_rejects_before_payload_read() {
    let mut input = Cursor::new(557u32.to_be_bytes().to_vec());
    let error =
        block_on(ReceiptCodec.read_request(&StreamProtocol::new(RECEIPT_PROTOCOL), &mut input))
            .unwrap_err();
    assert_eq!(
        error.kind(),
        std::io::ErrorKind::InvalidData,
        "closed receipt body is <=556, so absence of declared payload must not produce EOF"
    );
    assert_eq!(input.position(), 4);
}
#[path = "receipt_support/corpus.rs"]
mod corpus;
#[test]
fn valid_frames_are_exact_and_other_selected_namespace_is_rejected_before_read() {
    assert_eq!(corpus::matching("record", "shape").len(), 14);
    let raw = corpus::row("record", "status-committed");
    let mut framed = 556u32.to_be_bytes().to_vec();
    framed.extend(&raw);
    let protocol = StreamProtocol::new(RECEIPT_PROTOCOL);
    let record =
        block_on(ReceiptCodec.read_response(&protocol, &mut Cursor::new(framed.clone()))).unwrap();
    let mut output = Cursor::new(Vec::new());
    block_on(ReceiptCodec.write_response(&protocol, &mut output, record)).unwrap();
    assert_eq!(output.into_inner(), framed);
    let mut wrong = Cursor::new(framed.clone());
    assert_eq!(
        block_on(ReceiptCodec.read_request(
            &StreamProtocol::new("/nearfuture/peer/control/1"),
            &mut wrong
        ))
        .unwrap_err()
        .kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(wrong.position(), 0);
    framed.push(0);
    assert_eq!(
        block_on(ReceiptCodec.read_response(&protocol, &mut Cursor::new(framed)))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidData
    );
}
