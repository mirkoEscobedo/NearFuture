use super::{
    NotifyBody, NotifyContext, NotifyLimits, NotifyRecord, PROTOCOL,
    fields::{Reader, read_limits, read_proof, read_selector},
    validation::{context, validate},
};
use crate::PeerError;
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;
fn topic(r: &mut Reader<'_>) -> Result<(), PeerError> {
    if r.u8()? != 1 {
        return Err(PeerError::Unsupported);
    }
    Ok(())
}
pub fn decode_body(
    input: &[u8],
    actual_protocol: &str,
    limits: NotifyLimits,
) -> Result<NotifyRecord, PeerError> {
    if actual_protocol != PROTOCOL {
        return Err(PeerError::Unsupported);
    }
    limits.validate()?;
    if input.len() > usize::from(limits.frame) {
        return Err(PeerError::Limit);
    }
    let mut r = Reader { input, position: 0 };
    if &r.array::<12>()? != b"NF-NOTIFY-1\0" {
        return Err(PeerError::Malformed);
    }
    if r.u16()? != 1 {
        return Err(PeerError::Unsupported);
    }
    let kind = r.u8()?;
    if r.u8()? != 3 {
        return Err(PeerError::Unauthorized);
    }
    let ctx = NotifyContext {
        session: r.array()?,
        scope: Scope {
            universe: UniverseId::from_bytes(r.array()?),
            history: HistoryId::from_bytes(r.array()?),
        },
        ruleset: r.array()?,
        content: r.array()?,
    };
    context(ctx, kind == 1)?;
    let body = match kind {
        1 => NotifyBody::Hello {
            account: AccountId::from_bytes(r.array()?),
            device: DeviceId::from_bytes(r.array()?),
            nonce: r.array()?,
            required: r.u32()?,
            optional: r.u32()?,
            offered: read_limits(&mut r)?,
        },
        2 => NotifyBody::ServerHello {
            nonce: r.array()?,
            available: r.u32()?,
            selected_caps: r.u32()?,
            server_limits: read_limits(&mut r)?,
            selected: read_limits(&mut r)?,
            proof: read_proof(&mut r, ctx.scope)?,
        },
        3 => NotifyBody::ClientProof(read_proof(&mut r, ctx.scope)?),
        4 => NotifyBody::Finished(read_proof(&mut r, ctx.scope)?),
        5 => {
            let subscription = r.array()?;
            topic(&mut r)?;
            NotifyBody::BeginSubscribe {
                subscription,
                selector: read_selector(&mut r)?,
                nonce: r.array()?,
                minimum_membership: r.u64()?,
                lifetime: r.u16()?,
            }
        }
        6 => NotifyBody::SubscribeChallenge {
            subscription: r.array()?,
            client_nonce: r.array()?,
            server_nonce: r.array()?,
            frontier: r.u64()?,
            challenge: r.array()?,
        },
        7 => NotifyBody::ProveSubscribe {
            subscription: r.array()?,
            nonce: r.array()?,
            proof: read_proof(&mut r, ctx.scope)?,
        },
        8 => {
            let subscription = r.array()?;
            topic(&mut r)?;
            NotifyBody::Subscribed {
                subscription,
                selector: read_selector(&mut r)?,
                first_sequence: r.u64()?,
                lifetime: r.u16()?,
                proof: read_proof(&mut r, ctx.scope)?,
            }
        }
        9 => NotifyBody::Notice {
            subscription: r.array()?,
            sequence: r.u64()?,
            selector: read_selector(&mut r)?,
            nonce: r.array()?,
            proof: read_proof(&mut r, ctx.scope)?,
        },
        10 => {
            let subscription = r.array()?;
            let sequence = r.u64()?;
            let notice_digest = r.array()?;
            if r.u8()? != 1 {
                return Err(PeerError::Malformed);
            }
            NotifyBody::NoticeAck {
                subscription,
                sequence,
                notice_digest,
                proof: read_proof(&mut r, ctx.scope)?,
            }
        }
        _ => return Err(PeerError::Unsupported),
    };
    if r.position != input.len() {
        return Err(PeerError::Malformed);
    }
    let record = NotifyRecord { context: ctx, body };
    validate(&record, limits)?;
    Ok(record)
}
