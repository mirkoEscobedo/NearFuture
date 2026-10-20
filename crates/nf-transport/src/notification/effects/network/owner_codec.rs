use super::NotifyCodec;
use crate::notification::NotifyRecord;
use futures::{AsyncRead, AsyncWrite};
use libp2p::{StreamProtocol, request_response};
use std::{
    io,
    time::{Duration, Instant},
};
/// Owner-visible rejection is local data, never an encodable notification wire record.
#[derive(Clone, Debug)]
pub enum NotifyRequest {
    Record(Box<NotifyRecord>),
    Rejected,
}
/// The maintained backend drops some codec failures before pending request registration.
/// Return bounded local rejection data instead, so the actual owner observes peer/connection/ID.
#[derive(Clone, Default)]
pub struct NotifyOwnerCodec {
    inner: NotifyCodec,
}
impl request_response::Codec for NotifyOwnerCodec {
    type Protocol = StreamProtocol;
    type Request = NotifyRequest;
    type Response = NotifyRecord;
    async fn read_request<T>(
        &mut self,
        p: &Self::Protocol,
        stream: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // The fixed 4s wakeable cut precedes the backend's fixed 5s request deadline.
        // NotifyCodec still validates the length before allocation and requires exact EOF.
        let started = Instant::now();
        match tokio::time::timeout(Duration::from_secs(4), self.inner.read_request(p, stream)).await
        {
            Ok(Ok(record)) if started.elapsed() < Duration::from_secs(4) => {
                Ok(NotifyRequest::Record(Box::new(record)))
            }
            Ok(Ok(_)) => Ok(NotifyRequest::Rejected),
            Ok(Err(_)) | Err(_) => Ok(NotifyRequest::Rejected),
        }
    }
    async fn read_response<T>(
        &mut self,
        p: &Self::Protocol,
        stream: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.inner.read_response(p, stream).await
    }
    async fn write_request<T>(
        &mut self,
        p: &Self::Protocol,
        stream: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match request {
            NotifyRequest::Record(record) => self.inner.write_request(p, stream, *record).await,
            NotifyRequest::Rejected => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "local notification rejection is not wire data",
            )),
        }
    }
    async fn write_response<T>(
        &mut self,
        p: &Self::Protocol,
        stream: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.inner.write_response(p, stream, response).await
    }
}
