use crate::{FrameDecoder, IpcError, encode_frame};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
pub struct LoopbackListener {
    listener: TcpListener,
}
impl LoopbackListener {
    pub fn bind() -> Result<Self, IpcError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| IpcError::Io)?;
        listener.set_nonblocking(true).map_err(|_| IpcError::Io)?;
        Ok(Self { listener })
    }
    pub fn address(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("Bound listener retains address")
    }
    pub fn try_accept(&self, maximum: usize) -> Result<Option<FramePump>, IpcError> {
        match self.listener.accept() {
            Ok((stream, _)) => FramePump::new(stream, maximum).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(_) => Err(IpcError::Io),
        }
    }
}
/// One bounded inbound and outbound frame. All socket operations belong to a background owner.
/// Poll never waits for readiness. No DNS, game references, threads, process spawning or retries.
pub struct FramePump {
    stream: TcpStream,
    decoder: FrameDecoder,
    maximum: usize,
    outbound: Option<zeroize::Zeroizing<Vec<u8>>>,
    written: usize,
}
impl FramePump {
    pub fn new(stream: TcpStream, maximum: usize) -> Result<Self, IpcError> {
        if !stream
            .peer_addr()
            .map_err(|_| IpcError::Io)?
            .ip()
            .is_loopback()
            || !stream
                .local_addr()
                .map_err(|_| IpcError::Io)?
                .ip()
                .is_loopback()
        {
            return Err(IpcError::Unauthorized);
        }
        stream.set_nonblocking(true).map_err(|_| IpcError::Io)?;
        stream.set_nodelay(true).map_err(|_| IpcError::Io)?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(maximum)?,
            maximum,
            outbound: None,
            written: 0,
        })
    }
    pub fn activate(&mut self, session: &crate::AuthenticatedSession) -> Result<(), IpcError> {
        let maximum = session.limits().control_frame_bytes as usize;
        if self.outbound.is_some() || self.decoder.buffered_bytes() != 0 {
            return Err(IpcError::Limit);
        }
        self.decoder = FrameDecoder::new(maximum)?;
        self.maximum = maximum;
        Ok(())
    }
    pub fn restrict(&mut self, maximum: usize) -> Result<(), IpcError> {
        if maximum > self.maximum || self.outbound.is_some() || self.decoder.buffered_bytes() != 0 {
            return Err(IpcError::Limit);
        }
        self.decoder = FrameDecoder::new(maximum)?;
        self.maximum = maximum;
        Ok(())
    }
    pub fn send(&mut self, body: &[u8]) -> Result<(), IpcError> {
        if self.outbound.is_some() {
            return Err(IpcError::Backpressure);
        }
        self.outbound = Some(zeroize::Zeroizing::new(encode_frame(body, self.maximum)?));
        Ok(())
    }
    pub fn pending_write_bytes(&self) -> usize {
        self.outbound.as_ref().map_or(0, |v| v.len() - self.written)
    }
    /// Flushes only outbound bytes. Incoming frames stay unread under backpressure.
    pub fn flush(&mut self, write_budget: usize) -> Result<(), IpcError> {
        if write_budget == 0 || write_budget > 65536 {
            return Err(IpcError::Limit);
        }
        let mut writes = write_budget;
        while writes > 0 {
            let Some(frame) = &self.outbound else {
                break;
            };
            let end = frame.len().min(self.written + writes);
            match self.stream.write(&frame[self.written..end]) {
                Ok(0) => return Err(IpcError::Disconnected),
                Ok(n) => {
                    self.written += n;
                    writes -= n;
                    if self.written == frame.len() {
                        self.outbound = None;
                        self.written = 0;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => break,
                Err(_) => return Err(IpcError::Io),
            }
        }
        Ok(())
    }
    pub fn poll(
        &mut self,
        read_budget: usize,
        write_budget: usize,
    ) -> Result<Option<Vec<u8>>, IpcError> {
        if read_budget == 0 || read_budget > 65_536 || write_budget == 0 || write_budget > 65_536 {
            return Err(IpcError::Limit);
        }
        self.flush(write_budget)?;
        let mut reads = read_budget;
        let mut scratch = zeroize::Zeroizing::new([0; 4096]);
        while reads > 0 {
            let count = reads.min(scratch.len()).min(self.decoder.needed_bytes());
            match self.stream.read(&mut scratch[..count]) {
                Ok(0) => {
                    self.decoder.finish()?;
                    return Err(IpcError::Disconnected);
                }
                Ok(n) => {
                    reads -= n;
                    let decoded = self.decoder.push(&scratch[..n])?;
                    if decoded.consumed != n {
                        return Err(IpcError::Malformed);
                    }
                    if decoded.frame.is_some() {
                        return Ok(decoded.frame);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => break,
                Err(_) => return Err(IpcError::Io),
            }
        }
        Ok(None)
    }
}
