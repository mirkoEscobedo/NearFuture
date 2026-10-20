use crate::notification::{
    NotifyLimits, NotifyRecord, PROTOCOL, admit_frame_length, decode_body, encode_body,
};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::{StreamProtocol, request_response};
use std::io;
/// Fixed hard framing only; application state independently checks negotiated quotas and current SQL.
#[derive(Clone, Default)]
pub struct NotifyCodec;
impl NotifyCodec {
    fn invalid() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, "notification record rejected")
    }
    async fn read<T: AsyncRead + Unpin + Send>(
        &self,
        p: &StreamProtocol,
        stream: &mut T,
    ) -> io::Result<NotifyRecord> {
        if p.as_ref() != PROTOCOL {
            return Err(Self::invalid());
        }
        let mut prefix = [0; 4];
        stream.read_exact(&mut prefix).await?;
        let length =
            admit_frame_length(prefix, NotifyLimits::default()).map_err(|_| Self::invalid())?;
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).await?;
        let mut extra = [0; 1];
        if stream.read(&mut extra).await? != 0 {
            return Err(Self::invalid());
        }
        decode_body(&bytes, PROTOCOL, NotifyLimits::default()).map_err(|_| Self::invalid())
    }
    async fn write<T: AsyncWrite + Unpin + Send>(
        &self,
        p: &StreamProtocol,
        stream: &mut T,
        record: NotifyRecord,
    ) -> io::Result<()> {
        if p.as_ref() != PROTOCOL {
            return Err(Self::invalid());
        }
        let bytes =
            encode_body(&record, PROTOCOL, NotifyLimits::default()).map_err(|_| Self::invalid())?;
        let length = u32::try_from(bytes.len()).map_err(|_| Self::invalid())?;
        stream.write_all(&length.to_be_bytes()).await?;
        stream.write_all(&bytes).await?;
        stream.close().await
    }
}
impl request_response::Codec for NotifyCodec {
    type Protocol = StreamProtocol;
    type Request = NotifyRecord;
    type Response = NotifyRecord;
    async fn read_request<T>(&mut self, p: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read(p, io).await
    }
    async fn read_response<T>(
        &mut self,
        p: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read(p, io).await
    }
    async fn write_request<T>(
        &mut self,
        p: &Self::Protocol,
        io: &mut T,
        r: Self::Request,
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
        r: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write(p, io, r).await
    }
}
