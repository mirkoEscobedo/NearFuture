//! Fixed-width v1, big-endian integers. See docs/modules/probe-v1.md.
//! Decode borrowed bounded slices before copying any application payload.
use hmac::{Hmac, Mac};
use nf_modules::{Error, Invocation, Reply};
use sha2::Sha256;
use zeroize::Zeroizing;

pub const MAX_FRAME: usize = 65_536;
pub const HELLO_BYTES: usize = 80;
pub const ACCEPT_BYTES: usize = 88;
pub const REQUEST_HEADER: usize = 72;
pub const RESPONSE_HEADER: usize = 58;
pub const TAG_BYTES: usize = 32;
pub type Token = Zeroizing<[u8; 32]>;
type HmacSha256 = Hmac<Sha256>;

fn mac(key: &[u8], domain: &[u8], parts: &[&[u8]]) -> HmacSha256 {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts every key length");
    mac.update(domain);
    for part in parts {
        mac.update(part);
    }
    mac
}
fn sign(key: &[u8], domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    mac(key, domain, parts).finalize().into_bytes().into()
}
fn verify(key: &[u8], domain: &[u8], parts: &[&[u8]], tag: &[u8]) -> Result<(), Error> {
    mac(key, domain, parts)
        .verify_slice(tag)
        .map_err(|_| Error::Malformed)
}
fn fixed<const N: usize>(b: &[u8], offset: usize) -> [u8; N] {
    b[offset..offset + N]
        .try_into()
        .expect("caller validated fixed frame length")
}
fn u16_at(b: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(fixed(b, offset))
}
fn u32_at(b: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(fixed(b, offset))
}
fn u64_at(b: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(fixed(b, offset))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub request_bytes: u32,
    pub response_bytes: u32,
    pub timeout_ms: u32,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            request_bytes: MAX_FRAME as u32,
            response_bytes: 346,
            timeout_ms: 1000,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<Self, Error> {
        if !(136..=MAX_FRAME as u32).contains(&self.request_bytes)
            || !(122..=346).contains(&self.response_bytes)
            || !(1..=1000).contains(&self.timeout_ms)
        {
            return Err(Error::Limit);
        }
        Ok(self)
    }
    fn select(self) -> Self {
        let cap = Self::default();
        Self {
            request_bytes: self.request_bytes.min(cap.request_bytes),
            response_bytes: self.response_bytes.min(cap.response_bytes),
            timeout_ms: self.timeout_ms.min(cap.timeout_ms),
        }
    }
    fn write(self, b: &mut Vec<u8>) {
        for v in [self.request_bytes, self.response_bytes, self.timeout_ms] {
            b.extend_from_slice(&v.to_be_bytes());
        }
    }
    fn read(b: &[u8], offset: usize) -> Self {
        Self {
            request_bytes: u32_at(b, offset),
            response_bytes: u32_at(b, offset + 4),
            timeout_ms: u32_at(b, offset + 8),
        }
    }
}

pub struct Offer {
    pub minimum_version: u16,
    pub maximum_version: u16,
    pub requested: u32,
    pub required: u32,
    pub limits: Limits,
}
impl Default for Offer {
    fn default() -> Self {
        Self {
            minimum_version: 1,
            maximum_version: 1,
            requested: 1,
            required: 1,
            limits: Limits::default(),
        }
    }
}
pub struct Accepted {
    pub status: u16,
    pub available: u32,
    pub granted: u32,
    pub limits: Limits,
    pub channel: Channel,
    pub frame: Vec<u8>,
}
pub struct Channel {
    key: Token,
    pub limits: Limits,
    pub granted: u32,
}

/// The session/port were learned from the private child stdout, never untrusted discovery.
pub fn hello(
    token: &Token,
    session: [u8; 16],
    port: u16,
    nonce: [u8; 16],
    offer: &Offer,
) -> Vec<u8> {
    let mut b = Vec::with_capacity(HELLO_BYTES + TAG_BYTES);
    b.extend_from_slice(b"NFMHEL01");
    b.extend_from_slice(&session);
    b.extend_from_slice(&port.to_be_bytes());
    b.extend_from_slice(&offer.minimum_version.to_be_bytes());
    b.extend_from_slice(&offer.maximum_version.to_be_bytes());
    b.extend_from_slice(&0u16.to_be_bytes());
    b.extend_from_slice(&nonce);
    b.extend_from_slice(&offer.requested.to_be_bytes());
    b.extend_from_slice(&offer.required.to_be_bytes());
    offer.limits.write(&mut b);
    b.extend_from_slice(&[0; 12]);
    let tag = sign(token.as_ref(), b"NF-MODULE-HELLO-1\0", &[&b]);
    b.extend_from_slice(&tag);
    b
}

pub fn accept(
    token: &Token,
    session: [u8; 16],
    port: u16,
    available: u32,
    nonce: [u8; 16],
    h: &[u8],
) -> Result<Accepted, Error> {
    if h.len() != HELLO_BYTES + TAG_BYTES || &h[..8] != b"NFMHEL01" {
        return Err(Error::Malformed);
    }
    verify(
        token.as_ref(),
        b"NF-MODULE-HELLO-1\0",
        &[&h[..HELLO_BYTES]],
        &h[HELLO_BYTES..],
    )?;
    if fixed::<16>(h, 8) != session || u16_at(h, 24) != port {
        return Err(Error::StaleBinding);
    }
    if fixed::<16>(h, 32) == [0; 16] || h[30..32] != [0; 2] || h[68..80] != [0; 12] {
        return Err(Error::Malformed);
    }
    let requested = u32_at(h, 48);
    let required = u32_at(h, 52);
    let offered = Limits::read(h, 56);
    let status = if u16_at(h, 26) > 1 || u16_at(h, 28) < 1 || u16_at(h, 26) > u16_at(h, 28) {
        Error::UnsupportedVersion as u16
    } else if required & !requested != 0 || required & !available != 0 {
        Error::UnsupportedCapability as u16
    } else if offered.validate().is_err() {
        Error::Limit as u16
    } else {
        0
    };
    let limits = if status == 0 {
        offered.select()
    } else {
        Limits::default()
    };
    let granted = if status == 0 {
        requested & available
    } else {
        0
    };
    let mut b = Vec::with_capacity(ACCEPT_BYTES + TAG_BYTES);
    b.extend_from_slice(b"NFMACK01");
    b.extend_from_slice(&session);
    b.extend_from_slice(&1u16.to_be_bytes());
    b.extend_from_slice(&status.to_be_bytes());
    b.extend_from_slice(&available.to_be_bytes());
    b.extend_from_slice(&granted.to_be_bytes());
    limits.write(&mut b);
    b.extend_from_slice(&nonce);
    b.extend_from_slice(&[0; 24]);
    let key = Zeroizing::new(sign(token.as_ref(), b"NF-MODULE-KEY-1\0", &[h, &b]));
    let tag = sign(token.as_ref(), b"NF-MODULE-ACCEPT-1\0", &[h, &b]);
    b.extend_from_slice(&tag);
    Ok(Accepted {
        status,
        available,
        granted,
        limits,
        channel: Channel {
            key,
            limits,
            granted,
        },
        frame: b,
    })
}

pub fn finish(token: &Token, session: [u8; 16], h: &[u8], b: &[u8]) -> Result<Accepted, Error> {
    if h.len() != HELLO_BYTES + TAG_BYTES
        || b.len() != ACCEPT_BYTES + TAG_BYTES
        || &b[..8] != b"NFMACK01"
    {
        return Err(Error::Malformed);
    }
    verify(
        token.as_ref(),
        b"NF-MODULE-ACCEPT-1\0",
        &[h, &b[..ACCEPT_BYTES]],
        &b[ACCEPT_BYTES..],
    )?;
    if fixed::<16>(b, 8) != session
        || u16_at(b, 24) != 1
        || fixed::<16>(b, 48) == [0; 16]
        || b[64..88] != [0; 24]
    {
        return Err(Error::StaleBinding);
    }
    let status = u16_at(b, 26);
    let available = u32_at(b, 28);
    let granted = u32_at(b, 32);
    let limits = Limits::read(b, 36).validate()?;
    if status == 0
        && (granted != u32_at(h, 48) & available
            || u32_at(h, 52) & !granted != 0
            || limits != Limits::read(h, 56).select())
    {
        return Err(Error::Malformed);
    }
    let key = Zeroizing::new(sign(
        token.as_ref(),
        b"NF-MODULE-KEY-1\0",
        &[h, &b[..ACCEPT_BYTES]],
    ));
    Ok(Accepted {
        status,
        available,
        granted,
        limits,
        channel: Channel {
            key,
            limits,
            granted,
        },
        frame: b.to_vec(),
    })
}

pub struct Request<'a> {
    pub invocation: Invocation<'a>,
    pub sequence: u64,
    pub timeout_ms: u32,
}
impl Channel {
    pub fn encode_request(
        &self,
        r: &Invocation<'_>,
        sequence: u64,
        timeout_ms: u32,
    ) -> Result<Vec<u8>, Error> {
        let length = REQUEST_HEADER
            .checked_add(r.payload.len())
            .and_then(|n| n.checked_add(TAG_BYTES))
            .ok_or(Error::Limit)?;
        if length > self.limits.request_bytes as usize || r.payload.len() > nf_modules::MAX_PAYLOAD
        {
            return Err(Error::Limit);
        }
        let mut b = Vec::with_capacity(length);
        b.extend_from_slice(b"NFMREQ01");
        b.extend_from_slice(&r.session);
        b.extend_from_slice(&r.request_id);
        b.extend_from_slice(&r.campaign_generation.to_be_bytes());
        b.extend_from_slice(&sequence.to_be_bytes());
        b.extend_from_slice(&r.module.to_be_bytes());
        b.extend_from_slice(&r.operation.to_be_bytes());
        b.extend_from_slice(&r.api_version.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes());
        b.extend_from_slice(&r.required_capabilities.to_be_bytes());
        b.extend_from_slice(&timeout_ms.to_be_bytes());
        b.extend_from_slice(r.payload);
        let tag = sign(self.key.as_ref(), b"NF-MODULE-REQUEST-1\0", &[&b]);
        b.extend_from_slice(&tag);
        Ok(b)
    }
    pub fn decode_request<'a>(&self, b: &'a [u8]) -> Result<Request<'a>, Error> {
        if b.len() < REQUEST_HEADER + TAG_BYTES
            || b.len() > self.limits.request_bytes as usize
            || &b[..8] != b"NFMREQ01"
        {
            return Err(Error::Malformed);
        }
        let end = b.len() - TAG_BYTES;
        verify(
            self.key.as_ref(),
            b"NF-MODULE-REQUEST-1\0",
            &[&b[..end]],
            &b[end..],
        )?;
        if u16_at(b, 62) != 0 {
            return Err(Error::Malformed);
        }
        let timeout_ms = u32_at(b, 68);
        if timeout_ms == 0 || timeout_ms > self.limits.timeout_ms {
            return Err(Error::Limit);
        }
        Ok(Request {
            invocation: Invocation {
                session: fixed(b, 8),
                request_id: fixed(b, 24),
                campaign_generation: u64_at(b, 40),
                module: u16_at(b, 56),
                operation: u16_at(b, 58),
                api_version: u16_at(b, 60),
                required_capabilities: u32_at(b, 64),
                payload: &b[72..end],
            },
            sequence: u64_at(b, 48),
            timeout_ms,
        })
    }
    pub fn encode_reply(&self, r: &Reply, sequence: u64) -> Result<Vec<u8>, Error> {
        let (status, payload) = match &r.outcome {
            Ok(v) => (0, v.as_slice()),
            Err(e) => (*e as u16, &[][..]),
        };
        if RESPONSE_HEADER + TAG_BYTES + payload.len() > self.limits.response_bytes as usize {
            return Err(Error::Limit);
        }
        let mut b = Vec::with_capacity(RESPONSE_HEADER + TAG_BYTES + payload.len());
        b.extend_from_slice(b"NFMRSP01");
        b.extend_from_slice(&r.session);
        b.extend_from_slice(&r.request_id);
        b.extend_from_slice(&r.campaign_generation.to_be_bytes());
        b.extend_from_slice(&sequence.to_be_bytes());
        b.extend_from_slice(&status.to_be_bytes());
        b.extend_from_slice(payload);
        let tag = sign(self.key.as_ref(), b"NF-MODULE-REPLY-1\0", &[&b]);
        b.extend_from_slice(&tag);
        Ok(b)
    }
    /// Caller-owned expected identity/binding/sequence rejects delayed or substituted responses.
    pub fn decode_reply(
        &self,
        b: &[u8],
        request: &Invocation<'_>,
        sequence: u64,
    ) -> Result<Reply, Error> {
        if b.len() < RESPONSE_HEADER + TAG_BYTES
            || b.len() > self.limits.response_bytes as usize
            || &b[..8] != b"NFMRSP01"
        {
            return Err(Error::Malformed);
        }
        let end = b.len() - TAG_BYTES;
        verify(
            self.key.as_ref(),
            b"NF-MODULE-REPLY-1\0",
            &[&b[..end]],
            &b[end..],
        )?;
        if fixed::<16>(b, 8) != request.session
            || fixed::<16>(b, 24) != request.request_id
            || u64_at(b, 40) != request.campaign_generation
            || u64_at(b, 48) != sequence
        {
            return Err(Error::StaleBinding);
        }
        let status = u16_at(b, 56);
        let outcome = if status == 0 {
            Ok(b[58..end].to_vec())
        } else {
            if end != 58 {
                return Err(Error::Malformed);
            }
            Err(match status {
                1 => Error::Malformed,
                2 => Error::UnsupportedVersion,
                3 => Error::UnsupportedCapability,
                4 => Error::Limit,
                5 => Error::UnsupportedModule,
                6 => Error::UnsupportedOperation,
                7 => Error::Disabled,
                8 => Error::StaleBinding,
                9 => Error::Deadline,
                10 => Error::Cancelled,
                11 => Error::Unavailable,
                12 => Error::InvalidSnapshot,
                _ => return Err(Error::Malformed),
            })
        };
        Ok(Reply {
            session: request.session,
            request_id: request.request_id,
            campaign_generation: request.campaign_generation,
            outcome,
        })
    }
}
