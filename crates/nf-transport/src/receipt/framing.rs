use super::{HEADER_BYTES, MAX_BODY_BYTES, ReceiptRecord, decode_body, encode_body};
use crate::records::PeerLimits;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::{StreamProtocol, request_response};
use std::io;
pub const RECEIPT_PROTOCOL: &str = "/nearfuture/peer/control/2";
#[derive(Clone, Default)]
pub struct ReceiptCodec;
impl ReceiptCodec {
    fn invalid() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, "receipt record rejected")
    }
    async fn read<T: AsyncRead + Unpin + Send>(
        &self,
        p: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<ReceiptRecord> {
        if p.as_ref() != RECEIPT_PROTOCOL {
            return Err(Self::invalid());
        }
        let mut prefix = [0; 4];
        io.read_exact(&mut prefix).await?;
        let n = u32::from_be_bytes(prefix) as usize;
        if !(HEADER_BYTES..=MAX_BODY_BYTES).contains(&n) {
            return Err(Self::invalid());
        }
        let mut bytes = vec![0; n];
        io.read_exact(&mut bytes).await?;
        let mut extra = [0; 1];
        if io.read(&mut extra).await? != 0 {
            return Err(Self::invalid());
        }
        decode_body(&bytes, PeerLimits::default()).map_err(|_| Self::invalid())
    }
    async fn write<T: AsyncWrite + Unpin + Send>(
        &self,
        p: &StreamProtocol,
        io: &mut T,
        r: ReceiptRecord,
    ) -> io::Result<()> {
        if p.as_ref() != RECEIPT_PROTOCOL {
            return Err(Self::invalid());
        }
        let b = encode_body(&r, PeerLimits::default()).map_err(|_| Self::invalid())?;
        io.write_all(&(b.len() as u32).to_be_bytes()).await?;
        io.write_all(&b).await?;
        io.close().await
    }
}
impl request_response::Codec for ReceiptCodec {
    type Protocol = StreamProtocol;
    type Request = ReceiptRecord;
    type Response = ReceiptRecord;
    async fn read_request<T>(&mut self, p: &Self::Protocol, io: &mut T) -> io::Result<ReceiptRecord>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read(p, io).await
    }
    async fn read_response<T>(
        &mut self,
        p: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<ReceiptRecord>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read(p, io).await
    }
    async fn write_request<T>(
        &mut self,
        p: &Self::Protocol,
        io: &mut T,
        r: ReceiptRecord,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write(p, io, r).await
    }
    async fn write_response<T>(
        &mut self,
        p: &Self::Protocol,
        io: &mut T,
        r: ReceiptRecord,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write(p, io, r).await
    }
}
