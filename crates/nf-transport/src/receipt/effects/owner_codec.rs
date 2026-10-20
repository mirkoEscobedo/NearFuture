//! Owner-visible read refusals; the accepted receipt wire codec remains unchanged.
use crate::receipt::{ReceiptCodec, ReceiptRecord};
use futures::{AsyncRead, AsyncWrite};
use libp2p::{StreamProtocol, request_response::Codec};
use std::{
    io,
    time::{Duration, Instant},
};

/// Local backend input only. A refusal has no wire encoding or application authority.
#[derive(Clone, Debug)]
pub enum ReceiptRequest {
    Record(Box<ReceiptRecord>),
    Rejected,
}
impl From<ReceiptRecord> for ReceiptRequest {
    fn from(record: ReceiptRecord) -> Self {
        Self::Record(Box::new(record))
    }
}
#[derive(Clone, Default)]
pub struct ReceiptOwnerCodec;
impl Codec for ReceiptOwnerCodec {
    type Protocol = StreamProtocol;
    type Request = ReceiptRequest;
    type Response = ReceiptRecord;

    async fn read_request<T>(
        &mut self,
        protocol: &StreamProtocol,
        stream: &mut T,
    ) -> io::Result<ReceiptRequest>
    where
        T: AsyncRead + Unpin + Send,
    {
        // RR discards failures before registering an inbound request. Return bounded
        // local data so the owner receives the actual peer/connection/backend ID.
        // This wakeable budget expires before RR's five-second worker budget.
        let started = Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(4),
            ReceiptCodec.read_request(protocol, stream),
        )
        .await;
        // A synchronous read can finish after the budget before the timer driver runs.
        if started.elapsed() >= Duration::from_secs(4) {
            return Ok(ReceiptRequest::Rejected);
        }
        Ok(match result {
            Ok(Ok(record)) => record.into(),
            Ok(Err(_)) | Err(_) => ReceiptRequest::Rejected,
        })
    }
    async fn read_response<T>(
        &mut self,
        protocol: &StreamProtocol,
        stream: &mut T,
    ) -> io::Result<ReceiptRecord>
    where
        T: AsyncRead + Unpin + Send,
    {
        ReceiptCodec.read_response(protocol, stream).await
    }
    async fn write_request<T>(
        &mut self,
        protocol: &StreamProtocol,
        stream: &mut T,
        request: ReceiptRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match request {
            ReceiptRequest::Record(record) => {
                ReceiptCodec.write_request(protocol, stream, *record).await
            }
            ReceiptRequest::Rejected => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "receipt input rejected",
            )),
        }
    }
    async fn write_response<T>(
        &mut self,
        protocol: &StreamProtocol,
        stream: &mut T,
        response: ReceiptRecord,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        ReceiptCodec
            .write_response(protocol, stream, response)
            .await
    }
}
