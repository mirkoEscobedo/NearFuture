//! A real synchronous read can finish after the owner's timer budget.
use futures::{AsyncRead, io::Cursor};
use libp2p::{StreamProtocol, request_response::Codec};
use nf_transport::{
    receipt::RECEIPT_PROTOCOL,
    receipt_effects::{ReceiptOwnerCodec, ReceiptRequest},
};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
#[path = "receipt_support/corpus.rs"]
mod corpus;

struct BlockingRead {
    inner: Cursor<Vec<u8>>,
    first: bool,
}
impl AsyncRead for BlockingRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if self.first {
            self.first = false;
            // Real elapsed time; no clock override or invented runtime grant.
            std::thread::sleep(Duration::from_millis(4100));
        }
        Pin::new(&mut self.inner).poll_read(cx, bytes)
    }
}
#[tokio::test]
async fn completed_valid_read_after_budget_is_rejected() {
    let body = corpus::matching("record", "shape")
        .into_iter()
        .find(|(name, _)| *name == "hello")
        .unwrap()
        .1;
    let mut framed = (body.len() as u32).to_be_bytes().to_vec();
    framed.extend(body);
    let mut input = BlockingRead {
        inner: Cursor::new(framed),
        first: true,
    };
    let result = ReceiptOwnerCodec
        .read_request(&StreamProtocol::new(RECEIPT_PROTOCOL), &mut input)
        .await
        .unwrap();
    assert!(
        matches!(result, ReceiptRequest::Rejected),
        "completed input past the private four-second budget must not enter the owner as a record"
    );
}
