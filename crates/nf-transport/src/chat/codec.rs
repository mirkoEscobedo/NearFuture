use super::{ChatFrame, MAX_FRAME_BYTES, Refusal, WireContext};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use nf_identity::model::{DeviceProof, Scope};
use nf_store::chat::{
    ChatPolicy, ChatStoreError, IssuedChallenge, SignedMessage, codec, outbox::SignedChatReceipt,
};
const DOMAIN: &[u8; 15] = b"NF-CHAT-WIRE-1\0";

pub fn encode(frame: &ChatFrame) -> Result<Vec<u8>, PeerError> {
    let (tag, context) = match frame {
        ChatFrame::PostChallenge { context, .. } => (1, context),
        ChatFrame::PostProof { context, .. } => (2, context),
        ChatFrame::Issued { context, .. } => (3, context),
        ChatFrame::Delivered { context, .. } => (4, context),
        ChatFrame::Refused { context, .. } => (5, context),
    };
    if context.request.as_bytes() == &[0; 16] {
        return Err(PeerError::Malformed);
    }
    let mut out = Vec::with_capacity(2601);
    out.extend_from_slice(DOMAIN);
    out.push(tag);
    out.extend_from_slice(&context.policy_digest);
    out.extend_from_slice(context.request.as_bytes());
    match frame {
        ChatFrame::PostChallenge { signed, .. } => put_message(&mut out, context, signed)?,
        ChatFrame::PostProof {
            signed,
            ticket,
            proof,
            ..
        } => {
            put_message(&mut out, context, signed)?;
            validate_proof(signed, ticket, proof)?;
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
        ChatFrame::Issued { challenge, .. } => {
            if challenge.ticket == [0; 16] {
                return Err(PeerError::Malformed);
            }
            out.extend_from_slice(&challenge.ticket);
            out.extend_from_slice(&challenge.challenge);
            out.extend_from_slice(&challenge.membership_revision.to_be_bytes());
        }
        ChatFrame::Delivered { signed, .. } => {
            if context.policy_digest != signed.receipt.policy_digest {
                return Err(PeerError::Policy);
            }
            out.extend_from_slice(&signed.to_canonical_bytes().map_err(shape_error)?);
        }
        ChatFrame::Refused { reason, .. } => out.push(match reason {
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
/// Complete bounded canonical shape inspection; signatures remain unverified values.
/// This never verifies membership/proof/signature authority or writes any store.
pub fn decode(bytes: &[u8]) -> Result<ChatFrame, PeerError> {
    let frame = parse(bytes)?;
    if encode(&frame)?.as_slice() != bytes {
        return Err(PeerError::Malformed);
    }
    Ok(frame)
}
fn parse(bytes: &[u8]) -> Result<ChatFrame, PeerError> {
    if !(65..=MAX_FRAME_BYTES).contains(&bytes.len()) {
        return Err(PeerError::Limit);
    }
    let mut r = Reader { bytes, at: 0 };
    if r.take(15)? != DOMAIN {
        return Err(PeerError::Malformed);
    }
    let tag = r.fixed::<1>()?[0];
    let context = WireContext {
        policy_digest: r.fixed()?,
        request: RequestId::from_bytes(r.fixed()?),
    };
    if context.request.as_bytes() == &[0; 16] {
        return Err(PeerError::Malformed);
    }
    let frame = match tag {
        1 => ChatFrame::PostChallenge {
            context,
            signed: Box::new(read_message(&mut r, &context)?),
        },
        2 => {
            let signed = read_message(&mut r, &context)?;
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
            let proof = DeviceProof {
                scope,
                account,
                device,
                frontier,
                peer,
                challenge: r.fixed()?,
                signature: r.fixed()?,
            };
            validate_proof(&signed, &ticket, &proof)?;
            ChatFrame::PostProof {
                context,
                signed: Box::new(signed),
                ticket,
                proof: Box::new(proof),
            }
        }
        3 => {
            let challenge = IssuedChallenge {
                ticket: r.fixed()?,
                challenge: r.fixed()?,
                membership_revision: u64::from_be_bytes(r.fixed()?),
            };
            if challenge.ticket == [0; 16] {
                return Err(PeerError::Malformed);
            }
            ChatFrame::Issued { context, challenge }
        }
        4 => {
            let signed =
                SignedChatReceipt::from_canonical_bytes(r.take(323)?).map_err(shape_error)?;
            if context.policy_digest != signed.receipt.policy_digest {
                return Err(PeerError::Policy);
            }
            ChatFrame::Delivered {
                context,
                signed: Box::new(signed),
            }
        }
        5 => {
            let reason = match r.fixed::<1>()?[0] {
                1 => Refusal::Unsupported,
                2 => Refusal::Unauthorized,
                3 => Refusal::Limit,
                4 => Refusal::Replay,
                5 => Refusal::Conflict,
                6 => Refusal::Offline,
                _ => return Err(PeerError::Malformed),
            };
            ChatFrame::Refused { context, reason }
        }
        _ => return Err(PeerError::Malformed),
    };
    if r.at != bytes.len() {
        return Err(PeerError::Malformed);
    }
    Ok(frame)
}
fn put_message(
    out: &mut Vec<u8>,
    context: &WireContext,
    signed: &SignedMessage,
) -> Result<(), PeerError> {
    if context.policy_digest
        != codec::policy_digest(&ChatPolicy {
            scope: signed.message.scope,
        })
    {
        return Err(PeerError::Policy);
    }
    let bytes = codec::encode_signed_message(signed).map_err(shape_error)?;
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
    Ok(())
}
fn read_message(r: &mut Reader<'_>, context: &WireContext) -> Result<SignedMessage, PeerError> {
    let length = usize::from(u16::from_be_bytes(r.fixed()?));
    if !(174..=2221).contains(&length) {
        return Err(PeerError::Limit);
    }
    let signed = codec::decode_signed_message(r.take(length)?).map_err(shape_error)?;
    if context.policy_digest
        != codec::policy_digest(&ChatPolicy {
            scope: signed.message.scope,
        })
    {
        return Err(PeerError::Policy);
    }
    Ok(signed)
}
fn validate_proof(
    signed: &SignedMessage,
    ticket: &[u8; 16],
    proof: &DeviceProof,
) -> Result<(), PeerError> {
    if !(1..=128).contains(&proof.peer.len()) {
        return Err(PeerError::Limit);
    }
    if *ticket == [0; 16]
        || proof.scope != signed.message.scope
        || proof.account != signed.message.author.account
        || proof.device != signed.message.author.device
    {
        return Err(PeerError::Malformed);
    }
    // proof.peer remains a signature claim; a future adapter must compare it to observed Noise peer bytes.
    Ok(())
}
fn shape_error(error: ChatStoreError) -> PeerError {
    if error == ChatStoreError::Limit {
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
