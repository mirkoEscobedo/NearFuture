//! Opt-in bounded history profile. Decoded signatures remain unauthenticated values.
use super::{Refusal, WireContext};
use crate::PeerError;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use nf_identity::model::{DeviceProof, Scope};
use nf_store::chat::{
    Author, Channel, ChatPolicy, HistoryEntry, HistoryPage, HistoryQuery, IssuedChallenge, codec,
};
use std::io;
pub const PROTOCOL: &str = "/nearfuture/chat/history/1";
pub const MAX_FRAME_BYTES: usize = 4096;
pub const MAX_PAGE_ENTRIES: u16 = 1;
const DOMAIN: &[u8; 23] = b"NF-CHAT-HISTORY-WIRE-1\0";
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryFrame {
    Challenge {
        context: WireContext,
        query: HistoryQuery,
    },
    Proof {
        context: WireContext,
        query: HistoryQuery,
        ticket: [u8; 16],
        proof: Box<DeviceProof>,
    },
    Issued {
        context: WireContext,
        query: HistoryQuery,
        challenge: IssuedChallenge,
    },
    Page {
        context: WireContext,
        query: HistoryQuery,
        page: Box<HistoryPage>,
    },
    Refused {
        context: WireContext,
        query: HistoryQuery,
        reason: Refusal,
    },
}
impl HistoryFrame {
    pub fn context_query(&self) -> (WireContext, HistoryQuery) {
        match self {
            Self::Challenge { context, query }
            | Self::Proof { context, query, .. }
            | Self::Issued { context, query, .. }
            | Self::Page { context, query, .. }
            | Self::Refused { context, query, .. } => (*context, *query),
        }
    }
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Challenge { .. } | Self::Proof { .. })
    }
}
pub fn validate_query(context: WireContext, query: HistoryQuery) -> Result<(), PeerError> {
    if query.request.as_bytes() == &[0; 16] || query.request != context.request {
        return Err(PeerError::Malformed);
    }
    if query.limit != MAX_PAGE_ENTRIES {
        return Err(PeerError::Limit);
    }
    Ok(())
}
/// Exactly the existing store history binding, including every query field and local policy.
pub(super) fn binding(context: WireContext, query: HistoryQuery) -> Result<[u8; 32], PeerError> {
    use sha2::{Digest, Sha256};
    validate_query(context, query)?;
    let mut bytes = b"NF-CHAT-HISTORY-1\0".to_vec();
    bytes.extend_from_slice(&context.policy_digest);
    bytes.extend_from_slice(query.request.as_bytes());
    put_query(&mut bytes, query);
    Ok(Sha256::digest(bytes).into())
}
fn put_query(bytes: &mut Vec<u8>, query: HistoryQuery) {
    bytes.extend_from_slice(query.reader.account.as_bytes());
    bytes.extend_from_slice(query.reader.device.as_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&query.after_cursor.to_be_bytes());
    bytes.extend_from_slice(&query.limit.to_be_bytes());
}
pub fn validate_page(query: HistoryQuery, page: &HistoryPage) -> Result<(), PeerError> {
    if page.entries.len() > usize::from(MAX_PAGE_ENTRIES) {
        return Err(PeerError::Limit);
    }
    let next = match page.entries.as_slice() {
        [] => query.after_cursor,
        [entry] if entry.receiver_cursor > query.after_cursor => entry.receiver_cursor,
        _ => return Err(PeerError::Malformed),
    };
    if page.next_cursor != next {
        return Err(PeerError::Malformed);
    }
    Ok(())
}
pub fn encode(frame: &HistoryFrame) -> Result<Vec<u8>, PeerError> {
    let (context, query) = frame.context_query();
    validate_query(context, query)?;
    let tag = match frame {
        HistoryFrame::Challenge { .. } => 1,
        HistoryFrame::Proof { .. } => 2,
        HistoryFrame::Issued { .. } => 3,
        HistoryFrame::Page { .. } => 4,
        HistoryFrame::Refused { .. } => 5,
    };
    let mut out = Vec::with_capacity(2355);
    out.extend_from_slice(DOMAIN);
    out.push(tag);
    out.extend_from_slice(&context.policy_digest);
    out.extend_from_slice(context.request.as_bytes());
    put_query(&mut out, query);
    match frame {
        HistoryFrame::Challenge { .. } => {}
        HistoryFrame::Proof { ticket, proof, .. } => {
            if *ticket == [0; 16]
                || proof.account != query.reader.account
                || proof.device != query.reader.device
            {
                return Err(PeerError::Malformed);
            }
            if !(1..=128).contains(&proof.peer.len()) {
                return Err(PeerError::Limit);
            }
            if context.policy_digest != codec::policy_digest(&ChatPolicy { scope: proof.scope }) {
                return Err(PeerError::Policy);
            }
            out.extend_from_slice(ticket);
            out.extend_from_slice(proof.scope.universe.as_bytes());
            out.extend_from_slice(proof.scope.history.as_bytes());
            out.extend_from_slice(proof.account.as_bytes());
            out.extend_from_slice(proof.device.as_bytes());
            out.extend_from_slice(&proof.frontier.to_be_bytes());
            out.extend_from_slice(&(proof.peer.len() as u16).to_be_bytes());
            out.extend_from_slice(&proof.peer);
            out.extend_from_slice(&proof.challenge);
            out.extend_from_slice(&proof.signature);
        }
        HistoryFrame::Issued { challenge, .. } => {
            if challenge.ticket == [0; 16] {
                return Err(PeerError::Malformed);
            }
            out.extend_from_slice(&challenge.ticket);
            out.extend_from_slice(&challenge.challenge);
            out.extend_from_slice(&challenge.membership_revision.to_be_bytes());
        }
        HistoryFrame::Page { page, .. } => {
            validate_page(query, page)?;
            out.extend_from_slice(&page.next_cursor.to_be_bytes());
            out.push(page.entries.len() as u8);
            for entry in &page.entries {
                if context.policy_digest
                    != codec::policy_digest(&ChatPolicy {
                        scope: entry.signed.message.scope,
                    })
                {
                    return Err(PeerError::Policy);
                }
                let signed = codec::encode_signed_message(&entry.signed).map_err(shape_error)?;
                out.extend_from_slice(&entry.receiver_cursor.to_be_bytes());
                out.extend_from_slice(&(signed.len() as u16).to_be_bytes());
                out.extend_from_slice(&signed);
            }
        }
        HistoryFrame::Refused { reason, .. } => out.push(match reason {
            Refusal::Unsupported => 1,
            Refusal::Unauthorized => 2,
            Refusal::Limit => 3,
            Refusal::Replay => 4,
            Refusal::Conflict => 5,
            Refusal::Offline => 6,
        }),
    }
    if out.len() > MAX_FRAME_BYTES {
        return Err(PeerError::Limit);
    }
    Ok(out)
}
/// Canonical shape only; the foreground owner and client independently authorize every value.
pub fn decode(bytes: &[u8]) -> Result<HistoryFrame, PeerError> {
    if !(115..=MAX_FRAME_BYTES).contains(&bytes.len()) {
        return Err(PeerError::Limit);
    }
    let mut r = Reader { bytes, at: 0 };
    if r.take(23)? != DOMAIN {
        return Err(PeerError::Malformed);
    }
    let tag = r.fixed::<1>()?[0];
    let context = WireContext {
        policy_digest: r.fixed()?,
        request: RequestId::from_bytes(r.fixed()?),
    };
    let query = HistoryQuery {
        request: context.request,
        reader: Author {
            account: AccountId::from_bytes(r.fixed()?),
            device: DeviceId::from_bytes(r.fixed()?),
        },
        channel: match r.fixed::<1>()?[0] {
            1 => Channel::General,
            _ => return Err(PeerError::Malformed),
        },
        after_cursor: u64::from_be_bytes(r.fixed()?),
        limit: u16::from_be_bytes(r.fixed()?),
    };
    validate_query(context, query)?;
    let frame = match tag {
        1 => HistoryFrame::Challenge { context, query },
        2 => {
            let ticket = r.fixed()?;
            let scope = Scope {
                universe: UniverseId::from_bytes(r.fixed()?),
                history: HistoryId::from_bytes(r.fixed()?),
            };
            let account = AccountId::from_bytes(r.fixed()?);
            let device = DeviceId::from_bytes(r.fixed()?);
            let frontier = u64::from_be_bytes(r.fixed()?);
            let length = usize::from(u16::from_be_bytes(r.fixed()?));
            if !(1..=128).contains(&length) {
                return Err(PeerError::Limit);
            }
            let peer = r.take(length)?.to_vec();
            HistoryFrame::Proof {
                context,
                query,
                ticket,
                proof: Box::new(DeviceProof {
                    scope,
                    account,
                    device,
                    frontier,
                    peer,
                    challenge: r.fixed()?,
                    signature: r.fixed()?,
                }),
            }
        }
        3 => HistoryFrame::Issued {
            context,
            query,
            challenge: IssuedChallenge {
                ticket: r.fixed()?,
                challenge: r.fixed()?,
                membership_revision: u64::from_be_bytes(r.fixed()?),
            },
        },
        4 => {
            let next_cursor = u64::from_be_bytes(r.fixed()?);
            let count = r.fixed::<1>()?[0];
            if count > MAX_PAGE_ENTRIES as u8 {
                return Err(PeerError::Limit);
            }
            let mut entries = Vec::with_capacity(usize::from(count));
            for _ in 0..count {
                let receiver_cursor = u64::from_be_bytes(r.fixed()?);
                let length = usize::from(u16::from_be_bytes(r.fixed()?));
                if !(174..=2221).contains(&length) {
                    return Err(PeerError::Limit);
                }
                let signed = codec::decode_signed_message(r.take(length)?).map_err(shape_error)?;
                entries.push(HistoryEntry {
                    receiver_cursor,
                    signed,
                });
            }
            HistoryFrame::Page {
                context,
                query,
                page: Box::new(HistoryPage {
                    entries,
                    next_cursor,
                }),
            }
        }
        5 => HistoryFrame::Refused {
            context,
            query,
            reason: match r.fixed::<1>()?[0] {
                1 => Refusal::Unsupported,
                2 => Refusal::Unauthorized,
                3 => Refusal::Limit,
                4 => Refusal::Replay,
                5 => Refusal::Conflict,
                6 => Refusal::Offline,
                _ => return Err(PeerError::Malformed),
            },
        },
        _ => return Err(PeerError::Malformed),
    };
    if r.at != bytes.len() || encode(&frame)?.as_slice() != bytes {
        return Err(PeerError::Malformed);
    }
    Ok(frame)
}
fn shape_error(error: nf_store::chat::ChatStoreError) -> PeerError {
    if error == nf_store::chat::ChatStoreError::Limit {
        PeerError::Limit
    } else {
        PeerError::Malformed
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], PeerError> {
        let end = self.at.checked_add(length).ok_or(PeerError::Malformed)?;
        let part = self.bytes.get(self.at..end).ok_or(PeerError::Malformed)?;
        self.at = end;
        Ok(part)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], PeerError> {
        self.take(N)?.try_into().map_err(|_| PeerError::Malformed)
    }
}
#[derive(Clone, Default)]
pub struct HistoryCodec;
impl HistoryCodec {
    pub async fn read<T: AsyncRead + Unpin + Send>(&self, io: &mut T) -> io::Result<HistoryFrame> {
        let mut prefix = [0; 4];
        io.read_exact(&mut prefix).await?;
        let length = u32::from_be_bytes(prefix) as usize;
        if !(115..=MAX_FRAME_BYTES).contains(&length) {
            return Err(invalid());
        }
        let mut body = vec![0; length];
        io.read_exact(&mut body).await?;
        let mut extra = [0; 1];
        if io.read(&mut extra).await? != 0 {
            return Err(invalid());
        }
        decode(&body).map_err(|_| invalid())
    }
    pub async fn write<T: AsyncWrite + Unpin + Send>(
        &self,
        io: &mut T,
        frame: &HistoryFrame,
    ) -> io::Result<()> {
        let bytes = encode(frame).map_err(|_| invalid())?;
        io.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
        io.write_all(&bytes).await?;
        io.close().await
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "history frame rejected")
}
impl libp2p::request_response::Codec for HistoryCodec {
    type Protocol = libp2p::StreamProtocol;
    type Request = HistoryFrame;
    type Response = HistoryFrame;
    async fn read_request<T: AsyncRead + Unpin + Send>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<HistoryFrame> {
        let frame = self.read(io).await?;
        if !frame.is_request() {
            return Err(invalid());
        }
        Ok(frame)
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<HistoryFrame> {
        let frame = self.read(io).await?;
        if frame.is_request() {
            return Err(invalid());
        }
        Ok(frame)
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        frame: HistoryFrame,
    ) -> io::Result<()> {
        if !frame.is_request() {
            return Err(invalid());
        }
        self.write(io, &frame).await
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        frame: HistoryFrame,
    ) -> io::Result<()> {
        if frame.is_request() {
            return Err(invalid());
        }
        self.write(io, &frame).await
    }
}
