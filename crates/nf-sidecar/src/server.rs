use crate::protocol::{self, Channel, Token};
use nf_ipc::{FramePump, IpcError, LoopbackListener};
use nf_modules::{Context, Host};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

fn random<const N: usize>() -> Result<[u8; N], IpcError> {
    let mut b = [0; N];
    getrandom::fill(&mut b).map_err(|_| IpcError::Io)?;
    if b == [0; N] {
        return Err(IpcError::Io);
    }
    Ok(b)
}
pub fn fresh_session() -> Result<[u8; 16], IpcError> {
    random()
}

struct Connection {
    pump: FramePump,
    channel: Option<Channel>,
    deadline: Instant,
    started: Instant,
    next_sequence: u64,
    close_after_write: bool,
}
pub struct Server {
    listener: LoopbackListener,
    token: Token,
    host: Host,
    connections: Vec<Connection>,
}
impl Server {
    pub fn bind(token: Token, host: Host) -> Result<Self, IpcError> {
        if *token == [0; 32] {
            return Err(IpcError::Unauthorized);
        }
        Ok(Self {
            listener: LoopbackListener::bind()?,
            token,
            host,
            connections: Vec::with_capacity(4),
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.listener.address()
    }
    pub fn session(&self) -> [u8; 16] {
        self.host.session()
    }
    pub fn bind_campaign(&mut self, generation: u64) -> Result<(), nf_modules::Error> {
        self.host.bind_campaign(generation)?;
        self.connections.clear();
        Ok(())
    }
    pub fn stop(&mut self) {
        self.host.stop();
        self.connections.clear();
    }
    /// A bounded, nonblocking background turn. Expected client faults affect only that connection.
    pub fn poll(
        &mut self,
        now: Instant,
        cancelled: &AtomicBool,
        generation: &AtomicU64,
    ) -> Result<(), IpcError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            self.stop();
            return Ok(());
        }
        if self.connections.len() < 4
            && let Some(pump) = self.listener.try_accept(protocol::MAX_FRAME)?
        {
            self.connections.push(Connection {
                pump,
                channel: None,
                deadline: now + Duration::from_secs(2),
                started: now,
                next_sequence: 1,
                close_after_write: false,
            });
        }
        let mut i = 0;
        while i < self.connections.len() {
            let keep = Self::turn(
                &self.token,
                &self.host,
                self.address().port(),
                &mut self.connections[i],
                now,
                cancelled,
                generation,
            );
            if keep {
                i += 1;
            } else {
                self.connections.swap_remove(i);
            }
        }
        Ok(())
    }
    fn turn(
        token: &Token,
        host: &Host,
        port: u16,
        c: &mut Connection,
        now: Instant,
        cancelled: &AtomicBool,
        generation: &AtomicU64,
    ) -> bool {
        if now >= c.deadline
            || Instant::now() >= c.deadline
            || cancelled.load(Ordering::Acquire)
            || generation.load(Ordering::Acquire) != host.campaign_generation()
        {
            return false;
        }
        if c.pump.pending_write_bytes() != 0 {
            if c.pump.flush(4096).is_err() {
                return false;
            }
            if c.pump.pending_write_bytes() == 0 {
                if c.close_after_write {
                    return false;
                }
                if let Some(channel) = &c.channel
                    && c.pump
                        .restrict(channel.limits.request_bytes as usize)
                        .is_err()
                {
                    return false;
                }
                c.started = Instant::now();
                c.deadline = c.started
                    + Duration::from_millis(
                        c.channel
                            .as_ref()
                            .map_or(2000, |ch| u64::from(ch.limits.timeout_ms)),
                    );
            }
            return true;
        }
        let frame = match c.pump.poll(4096, 4096) {
            Ok(Some(b)) => zeroize::Zeroizing::new(b),
            Ok(None) => return true,
            Err(_) => return false,
        };
        if c.channel.is_none() {
            let nonce = match random() {
                Ok(n) => n,
                Err(_) => return false,
            };
            let accepted = match protocol::accept(
                token,
                host.session(),
                port,
                host.available(),
                nonce,
                &frame,
            ) {
                Ok(a) => a,
                Err(_) => return false,
            };
            if c.pump.send(&accepted.frame).is_err() {
                return false;
            }
            c.close_after_write = accepted.status != 0;
            c.channel = Some(accepted.channel);
            return true;
        }
        let channel = c.channel.as_ref().expect("checked channel");
        let r = match channel.decode_request(&frame) {
            Ok(r) => r,
            Err(_) => return false,
        };
        if r.sequence != c.next_sequence {
            return false;
        }
        c.next_sequence = match c.next_sequence.checked_add(1) {
            Some(n) => n,
            None => return false,
        };
        let deadline = Instant::now() + Duration::from_millis(u64::from(r.timeout_ms));
        let context = Context {
            deadline,
            cancelled,
        };
        let mut reply = host.dispatch(&r.invocation, channel.granted, &context);
        // A cooperative handler may have exceeded its deadline; never flush a late success.
        if context.check().is_err() {
            return false;
        }
        if reply.outcome.as_ref().is_ok_and(|v| {
            protocol::RESPONSE_HEADER + protocol::TAG_BYTES + v.len()
                > channel.limits.response_bytes as usize
        }) {
            reply.outcome = Err(nf_modules::Error::Limit);
        }
        let encoded = match channel.encode_reply(&reply, r.sequence) {
            Ok(b) => b,
            Err(_) => return false,
        };
        c.deadline = deadline;
        c.pump.send(&encoded).is_ok()
    }
}
