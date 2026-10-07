use super::{fields, reader::Reader, *};
use crate::{
    PeerError,
    records::{PeerContext, PeerLimits},
};
use nf_contract::identity::*;
use nf_identity::model::Scope;
/// Borrowed fixed shape admission only, never proof verification or installation.
pub fn decode_body(input: &[u8], limits: PeerLimits) -> Result<ReceiptRecord, PeerError> {
    limits.validate()?;
    if input.len() > MAX_BODY_BYTES || input.len() > limits.control_frame as usize {
        return Err(PeerError::Limit);
    }
    let mut r = Reader { input, position: 0 };
    if &r.array::<10>()? != b"NF-PEER-2\0" {
        return Err(PeerError::Malformed);
    }
    if r.u16()? != 2 {
        return Err(PeerError::Unsupported);
    }
    let kind = r.u8()?;
    if r.u8()? != 1 {
        return Err(PeerError::Unauthorized);
    }
    let context = PeerContext {
        session: r.array()?,
        scope: Scope {
            universe: UniverseId::from_bytes(fields::nonzero(r.array()?)?),
            history: HistoryId::from_bytes(fields::nonzero(r.array()?)?),
        },
        ruleset: r.array()?,
        content: r.array()?,
    };
    if (context.session == [0; 16]) != (kind == 1) {
        return Err(PeerError::Session);
    }
    let body = match kind {
        1 => {
            let account = AccountId::from_bytes(fields::nonzero(r.array()?)?);
            let device = DeviceId::from_bytes(fields::nonzero(r.array()?)?);
            let nonce = fields::nonzero(r.array()?)?;
            let required = r.u32()?;
            let optional = r.u32()?;
            if required != 1 {
                return Err(PeerError::Unsupported);
            }
            ReceiptBody::Hello {
                account,
                device,
                nonce,
                required,
                optional,
                offered: fields::read_limits(&mut r)?,
            }
        }
        2 => {
            let nonce = fields::nonzero(r.array()?)?;
            let available = r.u32()?;
            let selected_caps = r.u32()?;
            if available != 1 || selected_caps != 1 {
                return Err(PeerError::Unsupported);
            }
            let server_limits = fields::read_limits(&mut r)?;
            let selected = fields::read_limits(&mut r)?;
            if selected.negotiate(server_limits)? != selected {
                return Err(PeerError::Limit);
            }
            ReceiptBody::ServerHello {
                nonce,
                available,
                selected_caps,
                server_limits,
                selected,
                proof: fields::read_proof(&mut r, context.scope)?,
            }
        }
        3 => ReceiptBody::ClientProof(fields::read_proof(&mut r, context.scope)?),
        4 => ReceiptBody::Finished(fields::read_proof(&mut r, context.scope)?),
        5 => {
            let target = fields::target(&mut r)?;
            let nonce = fields::nonzero(r.array()?)?;
            ReceiptBody::Begin {
                target,
                nonce,
                minimum: SourceMinima {
                    membership_revision: r.u64()?,
                    event: EventSeq(r.u64()?),
                    store_revision: r.u64()?,
                },
            }
        }
        6 => ReceiptBody::Challenge {
            target: fields::target(&mut r)?,
            client_nonce: fields::nonzero(r.array()?)?,
            server_nonce: fields::nonzero(r.array()?)?,
            frontier: r.u64()?,
            challenge: r.array()?,
        },
        7 => ReceiptBody::Prove {
            request: RequestId::from_bytes(fields::nonzero(r.array()?)?),
            nonce: fields::nonzero(r.array()?)?,
            proof: fields::read_proof(&mut r, context.scope)?,
        },
        8 => ReceiptBody::Status {
            status: status::read(&mut r)?,
            proof: fields::read_proof(&mut r, context.scope)?,
        },
        9 => {
            let request = RequestId::from_bytes(fields::nonzero(r.array()?)?);
            let reason = match r.u8()? {
                1 => ReceiptUnsupportedReason::BindingConflict,
                2 => ReceiptUnsupportedReason::SourceBelowKnownMinima,
                _ => return Err(PeerError::Unsupported),
            };
            fields::profile(&mut r)?;
            ReceiptBody::Unsupported {
                request,
                reason,
                current: fields::current(&mut r)?,
                proof: fields::read_proof(&mut r, context.scope)?,
            }
        }
        _ => return Err(PeerError::Unsupported),
    };
    r.finish()?;
    Ok(ReceiptRecord { context, body })
}
