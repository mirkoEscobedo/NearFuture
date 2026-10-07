//! Fixed-length framed records. The prefix is admitted before allocating any payload buffer.
use crate::records::{Lane, PeerLimits, PeerRecord, decode_body, encode_body};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::io;
#[derive(Clone)]
pub struct PeerCodec {
    lane: Lane,
}
impl PeerCodec {
    pub fn new(lane: Lane) -> Self {
        Self { lane }
    }
    fn invalid() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, "peer record rejected")
    }
    async fn read<T: AsyncRead + Unpin + Send>(&self, io: &mut T) -> io::Result<PeerRecord> {
        let mut prefix = [0; 4];
        io.read_exact(&mut prefix).await?;
        let n = u32::from_be_bytes(prefix) as usize;
        let limits = PeerLimits::default();
        if n < 126 || n > limits.frame(self.lane) {
            return Err(Self::invalid());
        }
        let mut bytes = vec![0; n];
        io.read_exact(&mut bytes).await?;
        let mut extra = [0; 1];
        if io.read(&mut extra).await? != 0 {
            return Err(Self::invalid());
        }
        decode_body(&bytes, self.lane, limits).map_err(|_| Self::invalid())
    }
    async fn write<T: AsyncWrite + Unpin + Send>(
        &self,
        io: &mut T,
        record: PeerRecord,
    ) -> io::Result<()> {
        let bytes =
            encode_body(&record, self.lane, PeerLimits::default()).map_err(|_| Self::invalid())?;
        io.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
        io.write_all(&bytes).await?;
        io.close().await
    }
}
impl libp2p::request_response::Codec for PeerCodec {
    type Protocol = libp2p::StreamProtocol;
    type Request = PeerRecord;
    type Response = PeerRecord;
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read(io).await
    }
    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read(io).await
    }
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        record: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write(io, record).await
    }
    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        record: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write(io, record).await
    }
}
