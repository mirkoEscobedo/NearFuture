use crate::{
    framing::PeerCodec,
    records::{Lane, PeerRecord},
};
use futures::{AsyncRead, AsyncWrite};
use libp2p::{StreamProtocol, request_response};
use std::{
    io,
    time::{Duration, Instant},
};
/// Local refusal data has no wire encoding or application authority.
#[derive(Clone, Debug)]
pub enum PortalBulkRequest {
    Record(Box<PeerRecord>),
    Rejected,
}
#[derive(Clone)]
pub struct PortalBulkOwnerCodec {
    inner: PeerCodec,
}
impl Default for PortalBulkOwnerCodec {
    fn default() -> Self {
        Self {
            inner: PeerCodec::new(Lane::Bulk),
        }
    }
}
impl request_response::Codec for PortalBulkOwnerCodec {
    type Protocol = StreamProtocol;
    type Request = PortalBulkRequest;
    type Response = PeerRecord;
    async fn read_request<T>(
        &mut self,
        p: &StreamProtocol,
        s: &mut T,
    ) -> io::Result<PortalBulkRequest>
    where
        T: AsyncRead + Unpin + Send,
    {
        let start = Instant::now();
        match tokio::time::timeout(Duration::from_secs(4), self.inner.read_request(p, s)).await {
            Ok(Ok(r)) if start.elapsed() < Duration::from_secs(4) => {
                Ok(PortalBulkRequest::Record(Box::new(r)))
            }
            _ => Ok(PortalBulkRequest::Rejected),
        }
    }
    async fn read_response<T>(&mut self, p: &StreamProtocol, s: &mut T) -> io::Result<PeerRecord>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.inner.read_response(p, s).await
    }
    async fn write_request<T>(
        &mut self,
        p: &StreamProtocol,
        s: &mut T,
        r: PortalBulkRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match r {
            PortalBulkRequest::Record(r) => self.inner.write_request(p, s, *r).await,
            PortalBulkRequest::Rejected => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "local bulk refusal is not wire data",
            )),
        }
    }
    async fn write_response<T>(
        &mut self,
        p: &StreamProtocol,
        s: &mut T,
        r: PeerRecord,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.inner.write_response(p, s, r).await
    }
}
