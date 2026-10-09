use super::{ChatFrame, MAX_FRAME_BYTES, codec};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::io;
#[derive(Clone, Default)]
pub struct ChatCodec;
impl ChatCodec {
    pub async fn read<T: AsyncRead + Unpin + Send>(&self, io: &mut T) -> io::Result<ChatFrame> {
        let mut prefix = [0; 4];
        io.read_exact(&mut prefix).await?;
        let length = u32::from_be_bytes(prefix) as usize;
        if !(65..=MAX_FRAME_BYTES).contains(&length) {
            return Err(invalid());
        }
        let mut body = vec![0; length];
        io.read_exact(&mut body).await?;
        let mut extra = [0; 1];
        if io.read(&mut extra).await? != 0 {
            return Err(invalid());
        }
        codec::decode(&body).map_err(|_| invalid())
    }
    pub async fn write<T: AsyncWrite + Unpin + Send>(
        &self,
        io: &mut T,
        frame: &ChatFrame,
    ) -> io::Result<()> {
        let body = codec::encode(frame).map_err(|_| invalid())?;
        io.write_all(&(body.len() as u32).to_be_bytes()).await?;
        io.write_all(&body).await?;
        io.close().await
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "chat frame rejected")
}
impl libp2p::request_response::Codec for ChatCodec {
    type Protocol = libp2p::StreamProtocol;
    type Request = ChatFrame;
    type Response = ChatFrame;
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<ChatFrame>
    where
        T: AsyncRead + Unpin + Send,
    {
        let frame = self.read(io).await?;
        if !frame.is_request() {
            return Err(invalid());
        }
        Ok(frame)
    }
    async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<ChatFrame>
    where
        T: AsyncRead + Unpin + Send,
    {
        let frame = self.read(io).await?;
        if frame.is_request() {
            return Err(invalid());
        }
        Ok(frame)
    }
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        frame: ChatFrame,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        if !frame.is_request() {
            return Err(invalid());
        }
        self.write(io, &frame).await
    }
    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        frame: ChatFrame,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        if frame.is_request() {
            return Err(invalid());
        }
        self.write(io, &frame).await
    }
}
