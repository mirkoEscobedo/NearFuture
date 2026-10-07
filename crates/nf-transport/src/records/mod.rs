mod bulk;
pub use bulk::BulkDescriptor;
mod reason;
pub use reason::UnsupportedReason;
mod status;
pub use status::RetainedPhase;
mod encode;
pub(crate) mod fields;
pub use encode::encode_body;
mod model;
mod reader;
use crate::PeerError;
pub use model::*;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, RequestId, UniverseId};
use nf_identity::model::Scope;
use reader::Reader;
/// Closed borrowed shape admission. This performs no Noise, membership, proof or authority checks.
pub fn decode_body(
    input: &[u8],
    actual_lane: Lane,
    limits: PeerLimits,
) -> Result<PeerRecord, PeerError> {
    limits.validate()?;
    if input.len() > limits.frame(actual_lane) {
        return Err(PeerError::Limit);
    }
    let mut reader = Reader { input, position: 0 };
    if &reader.array::<10>()? != b"NF-PEER-1\0" {
        return Err(PeerError::Malformed);
    }
    if reader.u16()? != 1 {
        return Err(PeerError::Unsupported);
    }
    let kind = reader.u8()?;
    if reader.u8()? != actual_lane as u8 {
        return Err(PeerError::Unauthorized);
    }
    let context = PeerContext {
        session: reader.array()?,
        scope: Scope {
            universe: UniverseId::from_bytes(reader.array()?),
            history: HistoryId::from_bytes(reader.array()?),
        },
        ruleset: reader.array()?,
        content: reader.array()?,
    };
    if (context.session == [0; 16]) != (kind == 1) {
        return Err(PeerError::Session);
    }
    let body = match kind {
        1 => {
            let account = AccountId::from_bytes(reader.array()?);
            let device = DeviceId::from_bytes(reader.array()?);
            let nonce = reader.array()?;
            let required = reader.u32()?;
            let optional = reader.u32()?;
            if required & !3 != 0 || required & actual_lane as u32 == 0 {
                return Err(PeerError::Unsupported);
            }
            let offered = fields::read_limits(&mut reader)?;
            PeerBody::Hello {
                account,
                device,
                nonce,
                required,
                optional,
                offered,
            }
        }
        2 => {
            let nonce = reader.array()?;
            let available = reader.u32()?;
            let selected_caps = reader.u32()?;
            if available & !3 != 0
                || selected_caps & !available != 0
                || selected_caps & actual_lane as u32 == 0
            {
                return Err(PeerError::Unsupported);
            }
            let server_limits = fields::read_limits(&mut reader)?;
            let selected = fields::read_limits(&mut reader)?;
            if selected.negotiate(server_limits)? != selected {
                return Err(PeerError::Limit);
            }
            let proof = fields::read_proof(&mut reader, context.scope)?;
            PeerBody::ServerHello {
                nonce,
                available,
                selected_caps,
                server_limits,
                selected,
                proof,
            }
        }
        3 => PeerBody::ClientProof(fields::read_proof(&mut reader, context.scope)?),
        4 => PeerBody::Finished(fields::read_proof(&mut reader, context.scope)?),
        5 if actual_lane == Lane::Control => PeerBody::BeginQuery {
            request: RequestId::from_bytes(reader.array()?),
            nonce: reader.array()?,
            minimum_membership: reader.u64()?,
        },
        6 if actual_lane == Lane::Control => PeerBody::QueryChallenge {
            request: RequestId::from_bytes(reader.array()?),
            client_nonce: reader.array()?,
            server_nonce: reader.array()?,
            frontier: reader.u64()?,
            challenge: reader.array()?,
        },
        7 if actual_lane == Lane::Control => PeerBody::ProveQuery {
            request: RequestId::from_bytes(reader.array()?),
            nonce: reader.array()?,
            proof: fields::read_proof(&mut reader, context.scope)?,
        },
        8 if actual_lane == Lane::Control => PeerBody::RetainedStatus {
            request: RequestId::from_bytes(reader.array()?),
            phase: status::read(&mut reader)?,
            proof: fields::read_proof(&mut reader, context.scope)?,
        },
        9 => PeerBody::Unsupported {
            request: RequestId::from_bytes(reader.array()?),
            reason: UnsupportedReason::try_from(reader.u8()?)?,
            proof: fields::read_proof(&mut reader, context.scope)?,
        },
        10..=16 if actual_lane == Lane::Bulk => {
            bulk::read(kind, &mut reader, context.scope, limits)?
        }
        _ => return Err(PeerError::Unsupported),
    };
    reader.finish()?;
    Ok(PeerRecord { context, body })
}
/// Digest of exact header/body prefix, excluding final proof and external framing prefix. No authorization.
pub fn reply_prefix_digest(
    record: &PeerRecord,
    lane: Lane,
    limits: PeerLimits,
) -> Result<[u8; 32], PeerError> {
    use sha2::{Digest, Sha256};
    if !matches!(
        record.body,
        PeerBody::RetainedStatus { .. }
            | PeerBody::Unsupported { .. }
            | PeerBody::BulkVerified { .. }
            | PeerBody::BulkProgress { .. }
            | PeerBody::BulkReady { .. }
    ) {
        return Err(PeerError::Unsupported);
    }
    let b = encode_body(record, lane, limits)?;
    Ok(Sha256::digest(&b[..b.len() - 297]).into())
}
